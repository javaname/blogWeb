package mcp

import (
	"context"

	"blogWeb/internal/model"
	"blogWeb/internal/service"
)

func (s *Server) writeAudit(ctx context.Context, clientID *uint, transport, actionType, target, scope, status, requestID, actorIP, errorCode, payload string) {
	digest := ""
	if payload != "" {
		digest = service.HashDigest(payload)
	}
	_ = s.db.WithContext(ctx).Create(&model.MCPAuditLog{
		ClientID:      clientID,
		Transport:     transport,
		ActionType:    actionType,
		Target:        target,
		Scope:         scope,
		Status:        status,
		RequestID:     requestID,
		ActorIP:       actorIP,
		ErrorCode:     errorCode,
		PayloadDigest: digest,
	}).Error
}
