package main

import (
	"context"
	"flag"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"blogWeb/internal/app"
)

func main() {
	if err := run(); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}

func run() error {
	command := "serve-web"
	args := []string{}
	if len(os.Args) > 1 {
		command = os.Args[1]
	}
	if len(os.Args) > 2 {
		args = os.Args[2:]
	}

	switch command {
	case "serve-web":
		return runServeWeb(args)
	case "serve-mcp":
		return runServeMCP(args)
	case "mcp":
		return runMCPCommand(args)
	default:
		return fmt.Errorf("unknown command %q", command)
	}
}

func runServeWeb(args []string) error {
	fs := flag.NewFlagSet("serve-web", flag.ContinueOnError)
	configPath := fs.String("config", "config.yaml", "config file path")
	if err := fs.Parse(args); err != nil {
		return err
	}

	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	application, err := app.Bootstrap(ctx, *configPath, true)
	if err != nil {
		return err
	}

	server := &http.Server{
		Addr:              fmt.Sprintf(":%d", application.Config.Server.Port),
		Handler:           application.WebRouter(),
		ReadHeaderTimeout: 10 * time.Second,
	}

	go func() {
		<-ctx.Done()
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		_ = server.Shutdown(shutdownCtx)
	}()

	application.Logger.Info("web server started", "addr", server.Addr)
	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		return err
	}
	return nil
}

func runServeMCP(args []string) error {
	fs := flag.NewFlagSet("serve-mcp", flag.ContinueOnError)
	configPath := fs.String("config", "config.yaml", "config file path")
	transport := fs.String("transport", "stdio", "stdio or http")
	if err := fs.Parse(args); err != nil {
		return err
	}

	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	application, err := app.Bootstrap(ctx, *configPath, true)
	if err != nil {
		return err
	}

	switch strings.ToLower(*transport) {
	case "stdio":
		return application.MCP.ServeStdio(ctx, os.Stdin, os.Stdout, os.Stderr)
	case "http":
		server := &http.Server{
			Addr:              application.Config.MCP.HTTPAddr,
			Handler:           application.MCPHTTPHandler(),
			ReadHeaderTimeout: 10 * time.Second,
		}
		go func() {
			<-ctx.Done()
			shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
			defer cancel()
			_ = server.Shutdown(shutdownCtx)
		}()
		application.Logger.Info("mcp http server started", "addr", server.Addr, "path", application.Config.MCP.HTTPPath)
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			return err
		}
		return nil
	default:
		return fmt.Errorf("unsupported mcp transport %q", *transport)
	}
}

func runMCPCommand(args []string) error {
	if len(args) == 0 {
		return fmt.Errorf("missing mcp subcommand")
	}

	switch args[0] {
	case "issue-token":
		fs := flag.NewFlagSet("issue-token", flag.ContinueOnError)
		configPath := fs.String("config", "config.yaml", "config file path")
		name := fs.String("name", "", "client name")
		scopes := fs.String("scopes", "", "comma-separated scopes")
		transport := fs.String("transport", "http", "http, stdio or both")
		if err := fs.Parse(args[1:]); err != nil {
			return err
		}
		if *name == "" || *scopes == "" {
			return fmt.Errorf("name and scopes are required")
		}
		ctx := context.Background()
		application, err := app.Bootstrap(ctx, *configPath, false)
		if err != nil {
			return err
		}
		token, err := application.MCP.IssueToken(ctx, *name, strings.Split(*scopes, ","), *transport)
		if err != nil {
			return err
		}
		fmt.Printf("name=%s\ntransport=%s\ntoken=%s\n", *name, *transport, token)
		return nil
	case "revoke-token":
		fs := flag.NewFlagSet("revoke-token", flag.ContinueOnError)
		configPath := fs.String("config", "config.yaml", "config file path")
		name := fs.String("name", "", "client name")
		if err := fs.Parse(args[1:]); err != nil {
			return err
		}
		if *name == "" {
			return fmt.Errorf("name is required")
		}
		ctx := context.Background()
		application, err := app.Bootstrap(ctx, *configPath, false)
		if err != nil {
			return err
		}
		return application.MCP.RevokeToken(ctx, *name)
	default:
		return fmt.Errorf("unknown mcp subcommand %q", args[0])
	}
}
