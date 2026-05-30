package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"strings"

	"blogWeb/internal/service"
)

func (s *Server) serveStdio(ctx context.Context, input io.Reader, output io.Writer, stderr io.Writer) error {
	decoder := json.NewDecoder(input)
	encoder := json.NewEncoder(output)
	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
		}

		var request jsonRPCRequest
		if err := decoder.Decode(&request); err != nil {
			if err == io.EOF {
				return nil
			}
			_, _ = fmt.Fprintln(stderr, err.Error())
			_ = encoder.Encode(jsonRPCResponse{
				JSONRPC: "2.0",
				Error: &jsonRPCError{
					Code:    400,
					Message: "JSON-RPC 请求格式错误",
				},
			})
			continue
		}

		if !s.config.MCP.StdioWriteEnabled {
			if err := s.rejectStdioWriteRequest(&request); err != nil {
				if appErr, ok := service.IsAppError(err); ok {
					_ = encoder.Encode(jsonRPCResponse{
						JSONRPC: "2.0",
						ID:      request.ID,
						Error: &jsonRPCError{
							Code:    appErr.StatusCode,
							Message: appErr.Message,
							Data:    map[string]any{"code": appErr.Code},
						},
					})
					continue
				}
				_, _ = fmt.Fprintln(stderr, err.Error())
				_ = encoder.Encode(jsonRPCResponse{
					JSONRPC: "2.0",
					ID:      request.ID,
					Error: &jsonRPCError{
						Code:    500,
						Message: err.Error(),
					},
				})
				continue
			}
		}

		result, err := s.dispatchRPCWithAccess(ctx, &request, s.config.MCP.StdioWriteEnabled)
		if err != nil {
			if appErr, ok := service.IsAppError(err); ok {
				_ = encoder.Encode(jsonRPCResponse{
					JSONRPC: "2.0",
					ID:      request.ID,
					Error: &jsonRPCError{
						Code:    appErr.StatusCode,
						Message: appErr.Message,
						Data:    map[string]any{"code": appErr.Code},
					},
				})
				continue
			}
			_, _ = fmt.Fprintln(stderr, err.Error())
			_ = encoder.Encode(jsonRPCResponse{
				JSONRPC: "2.0",
				ID:      request.ID,
				Error: &jsonRPCError{
					Code:    500,
					Message: err.Error(),
				},
			})
			continue
		}
		_ = encoder.Encode(jsonRPCResponse{
			JSONRPC: "2.0",
			ID:      request.ID,
			Result:  result,
		})
	}
}

func (s *Server) rejectStdioWriteRequest(request *jsonRPCRequest) error {
	if request == nil {
		return nil
	}
	switch request.Method {
	case "resources/read":
		var params struct {
			URI string `json:"uri"`
		}
		if err := json.Unmarshal(request.Params, &params); err != nil {
			return nil
		}
		if strings.HasPrefix(params.URI, "blog://drafts/") {
			return service.NewAppError(403, "forbidden_scope", "stdio 写能力默认关闭，请开启 mcp.stdio_write_enabled")
		}
	case "tools/call":
		var params struct {
			Name string `json:"name"`
		}
		if err := json.Unmarshal(request.Params, &params); err != nil {
			return nil
		}
		if isWriteTool(params.Name) {
			return service.NewAppError(403, "forbidden_scope", "stdio 写能力默认关闭，请开启 mcp.stdio_write_enabled")
		}
	}
	return nil
}
