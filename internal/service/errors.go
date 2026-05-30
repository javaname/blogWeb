package service

import "errors"

type AppError struct {
	StatusCode int
	Code       string
	Message    string
}

func (e *AppError) Error() string {
	return e.Message
}

func NewAppError(statusCode int, code, message string) *AppError {
	return &AppError{
		StatusCode: statusCode,
		Code:       code,
		Message:    message,
	}
}

func IsAppError(err error) (*AppError, bool) {
	var appErr *AppError
	if errors.As(err, &appErr) {
		return appErr, true
	}
	return nil, false
}
