package handler

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"mime/multipart"
	"net/http"
	"net/http/httptest"
	"strconv"
	"strings"
	"testing"
	"time"

	"blogWeb/internal/service"
	"blogWeb/internal/testutil"
)

type fakeRegistrationEmailSender struct {
	sent []service.RegistrationEmail
}

func (s *fakeRegistrationEmailSender) SendRegistrationCode(ctx context.Context, email, code string, ttl time.Duration) error {
	s.sent = append(s.sent, service.RegistrationEmail{
		Email: email,
		Code:  code,
		TTL:   ttl,
	})
	return nil
}

func TestPublicArticleDetailRedirectsHistoricalSlug(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	category := testutil.SeedCategory(t, app, "Tech", "tech")
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:      "First Title",
		Content:    "# body",
		CategoryID: &category.ID,
		Status:     "published",
	})

	newTitle := "Updated Title"
	if _, err := app.Articles.Update(t.Context(), article.ID, service.UpdateArticleInput{Title: &newTitle}); err != nil {
		t.Fatalf("update article: %v", err)
	}

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	req := httptest.NewRequest(http.MethodGet, "/api/articles/"+article.Slug, nil)
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusMovedPermanently {
		t.Fatalf("expected 301 redirect, got %d", resp.Code)
	}
}

func TestLikeEndpointsAndBatchStatus(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:   "Public article",
		Content: "# body",
		Status:  "published",
	})

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	body := strings.NewReader(`{"action":"like"}`)
	req := httptest.NewRequest(http.MethodPost, "/api/articles/"+article.Slug+"/like", body)
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Anonymous-Id", "anon-1")
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusOK {
		t.Fatalf("expected like 200, got %d: %s", resp.Code, resp.Body.String())
	}

	batchReq := httptest.NewRequest(http.MethodPost, "/api/likes/batch", strings.NewReader(`{"article_slugs":["`+article.Slug+`"]}`))
	batchReq.Header.Set("Content-Type", "application/json")
	batchReq.Header.Set("X-Anonymous-Id", "anon-1")
	batchResp := httptest.NewRecorder()
	router.ServeHTTP(batchResp, batchReq)
	if batchResp.Code != http.StatusOK {
		t.Fatalf("expected batch status 200, got %d", batchResp.Code)
	}
	if !strings.Contains(batchResp.Body.String(), `"liked_map":{"`+article.Slug+`":true}`) {
		t.Fatalf("unexpected batch body: %s", batchResp.Body.String())
	}
}

func TestLikeEndpointsUseAnonymousCookieWhenHeaderMissing(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:   "Cookie reader state",
		Content: "# body",
		Status:  "published",
	})

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	body := strings.NewReader(`{"action":"like"}`)
	req := httptest.NewRequest(http.MethodPost, "/api/articles/"+article.Slug+"/like", body)
	req.Header.Set("Content-Type", "application/json")
	req.AddCookie(&http.Cookie{Name: "anonymous_id", Value: "anon-cookie-1"})
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusOK {
		t.Fatalf("expected cookie-backed like 200, got %d: %s", resp.Code, resp.Body.String())
	}

	batchReq := httptest.NewRequest(http.MethodPost, "/api/likes/batch", strings.NewReader(`{"article_slugs":["`+article.Slug+`"]}`))
	batchReq.Header.Set("Content-Type", "application/json")
	batchReq.AddCookie(&http.Cookie{Name: "anonymous_id", Value: "anon-cookie-1"})
	batchResp := httptest.NewRecorder()
	router.ServeHTTP(batchResp, batchReq)
	if batchResp.Code != http.StatusOK {
		t.Fatalf("expected cookie-backed batch status 200, got %d: %s", batchResp.Code, batchResp.Body.String())
	}
	if !strings.Contains(batchResp.Body.String(), `"liked_map":{"`+article.Slug+`":true}`) {
		t.Fatalf("unexpected batch body: %s", batchResp.Body.String())
	}
}

func TestEmailRegistrationSendsCodeAndCreatesUser(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	sender := &fakeRegistrationEmailSender{}
	app.Auth.SetRegistrationEmailSender(sender)
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()

	codeReq := httptest.NewRequest(http.MethodPost, "/api/auth/register/code", strings.NewReader(`{"email":"Reader@Example.com"}`))
	codeReq.Header.Set("Content-Type", "application/json")
	codeResp := httptest.NewRecorder()
	router.ServeHTTP(codeResp, codeReq)
	if codeResp.Code != http.StatusCreated {
		t.Fatalf("expected code send 201, got %d: %s", codeResp.Code, codeResp.Body.String())
	}
	if len(sender.sent) != 1 {
		t.Fatalf("expected one registration email, got %d", len(sender.sent))
	}
	if sender.sent[0].Email != "reader@example.com" {
		t.Fatalf("expected normalized email, got %+v", sender.sent[0])
	}
	if len(sender.sent[0].Code) != 6 {
		t.Fatalf("expected 6 digit code, got %q", sender.sent[0].Code)
	}
	if !strings.Contains(codeResp.Body.String(), `"sent":true`) {
		t.Fatalf("expected sent response, got %s", codeResp.Body.String())
	}

	registerBody := `{"email":"reader@example.com","code":"` + sender.sent[0].Code + `","password":"reader-password","confirm_password":"reader-password"}`
	registerReq := httptest.NewRequest(http.MethodPost, "/api/auth/register", strings.NewReader(registerBody))
	registerReq.Header.Set("Content-Type", "application/json")
	registerResp := httptest.NewRecorder()
	router.ServeHTTP(registerResp, registerReq)
	if registerResp.Code != http.StatusCreated {
		t.Fatalf("expected register 201, got %d: %s", registerResp.Code, registerResp.Body.String())
	}
	if !strings.Contains(registerResp.Body.String(), `"email":"reader@example.com"`) || !strings.Contains(registerResp.Body.String(), `"role":"user"`) {
		t.Fatalf("unexpected register response: %s", registerResp.Body.String())
	}

	loginReq := httptest.NewRequest(http.MethodPost, "/api/admin/login", strings.NewReader(`{"username":"reader@example.com","password":"reader-password"}`))
	loginReq.Header.Set("Content-Type", "application/json")
	loginResp := httptest.NewRecorder()
	router.ServeHTTP(loginResp, loginReq)
	if loginResp.Code != http.StatusOK {
		t.Fatalf("expected email login 200, got %d: %s", loginResp.Code, loginResp.Body.String())
	}
}

func TestEmailRegistrationRejectsInvalidOrExpiredCode(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	sender := &fakeRegistrationEmailSender{}
	app.Auth.SetRegistrationEmailSender(sender)
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()

	codeReq := httptest.NewRequest(http.MethodPost, "/api/auth/register/code", strings.NewReader(`{"email":"reader2@example.com"}`))
	codeReq.Header.Set("Content-Type", "application/json")
	codeResp := httptest.NewRecorder()
	router.ServeHTTP(codeResp, codeReq)
	if codeResp.Code != http.StatusCreated {
		t.Fatalf("expected code send 201, got %d: %s", codeResp.Code, codeResp.Body.String())
	}

	wrongReq := httptest.NewRequest(http.MethodPost, "/api/auth/register", strings.NewReader(`{"email":"reader2@example.com","code":"000000","password":"reader-password","confirm_password":"reader-password"}`))
	wrongReq.Header.Set("Content-Type", "application/json")
	wrongResp := httptest.NewRecorder()
	router.ServeHTTP(wrongResp, wrongReq)
	if wrongResp.Code != http.StatusBadRequest {
		t.Fatalf("expected wrong code 400, got %d: %s", wrongResp.Code, wrongResp.Body.String())
	}
	if !strings.Contains(wrongResp.Body.String(), "invalid_verification_code") {
		t.Fatalf("expected invalid verification code error, got %s", wrongResp.Body.String())
	}

	if err := app.DB.Exec("UPDATE email_verification_codes SET expires_at = ? WHERE email = ?", time.Now().Add(-time.Minute), "reader2@example.com").Error; err != nil {
		t.Fatalf("expire code: %v", err)
	}
	expiredReq := httptest.NewRequest(http.MethodPost, "/api/auth/register", strings.NewReader(`{"email":"reader2@example.com","code":"`+sender.sent[0].Code+`","password":"reader-password","confirm_password":"reader-password"}`))
	expiredReq.Header.Set("Content-Type", "application/json")
	expiredResp := httptest.NewRecorder()
	router.ServeHTTP(expiredResp, expiredReq)
	if expiredResp.Code != http.StatusBadRequest {
		t.Fatalf("expected expired code 400, got %d: %s", expiredResp.Code, expiredResp.Body.String())
	}
}

func TestEmailRegistrationRejectsDuplicateEmail(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	sender := &fakeRegistrationEmailSender{}
	app.Auth.SetRegistrationEmailSender(sender)
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()

	codeReq := httptest.NewRequest(http.MethodPost, "/api/auth/register/code", strings.NewReader(`{"email":"duplicate@example.com"}`))
	codeReq.Header.Set("Content-Type", "application/json")
	codeResp := httptest.NewRecorder()
	router.ServeHTTP(codeResp, codeReq)
	if codeResp.Code != http.StatusCreated {
		t.Fatalf("expected code send 201, got %d: %s", codeResp.Code, codeResp.Body.String())
	}

	registerReq := httptest.NewRequest(http.MethodPost, "/api/auth/register", strings.NewReader(`{"email":"duplicate@example.com","code":"`+sender.sent[0].Code+`","password":"reader-password","confirm_password":"reader-password"}`))
	registerReq.Header.Set("Content-Type", "application/json")
	registerResp := httptest.NewRecorder()
	router.ServeHTTP(registerResp, registerReq)
	if registerResp.Code != http.StatusCreated {
		t.Fatalf("expected register 201, got %d: %s", registerResp.Code, registerResp.Body.String())
	}

	duplicateReq := httptest.NewRequest(http.MethodPost, "/api/auth/register/code", strings.NewReader(`{"email":"duplicate@example.com"}`))
	duplicateReq.Header.Set("Content-Type", "application/json")
	duplicateResp := httptest.NewRecorder()
	router.ServeHTTP(duplicateResp, duplicateReq)
	if duplicateResp.Code != http.StatusConflict {
		t.Fatalf("expected duplicate email 409, got %d: %s", duplicateResp.Code, duplicateResp.Body.String())
	}
}

func TestNewsletterSubscribeStoresEmail(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()

	req := httptest.NewRequest(http.MethodPost, "/api/newsletter/subscribe", strings.NewReader(`{"email":"reader@example.test"}`))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Anonymous-Id", "anon-newsletter-1")
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusCreated {
		t.Fatalf("expected subscribe 201, got %d: %s", resp.Code, resp.Body.String())
	}

	var payload struct {
		Subscribed bool   `json:"subscribed"`
		Email      string `json:"email"`
	}
	if err := json.Unmarshal(resp.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if !payload.Subscribed || payload.Email != "reader@example.test" {
		t.Fatalf("unexpected subscribe payload: %+v", payload)
	}

	var count int64
	if err := app.DB.Table("newsletter_subscriptions").Where("email = ? AND anonymous_id = ?", "reader@example.test", "anon-newsletter-1").Count(&count).Error; err != nil {
		t.Fatalf("count subscriptions: %v", err)
	}
	if count != 1 {
		t.Fatalf("expected stored subscription, got %d", count)
	}
}

func TestBookmarkAndAuthorFollowEndpointsToggleReaderState(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:   "Interactive article",
		Content: "# body",
		Status:  "published",
	})
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()

	bookmarkReq := httptest.NewRequest(http.MethodPost, "/api/articles/"+article.Slug+"/bookmark", strings.NewReader(`{"action":"bookmark"}`))
	bookmarkReq.Header.Set("Content-Type", "application/json")
	bookmarkReq.Header.Set("X-Anonymous-Id", "anon-reader-state")
	bookmarkResp := httptest.NewRecorder()
	router.ServeHTTP(bookmarkResp, bookmarkReq)
	if bookmarkResp.Code != http.StatusOK {
		t.Fatalf("expected bookmark 200, got %d: %s", bookmarkResp.Code, bookmarkResp.Body.String())
	}
	if !strings.Contains(bookmarkResp.Body.String(), `"bookmarked":true`) {
		t.Fatalf("expected bookmarked response, got %s", bookmarkResp.Body.String())
	}

	followReq := httptest.NewRequest(http.MethodPost, "/api/authors/"+strconv.Itoa(int(article.AuthorID))+"/follow", strings.NewReader(`{"action":"follow"}`))
	followReq.Header.Set("Content-Type", "application/json")
	followReq.Header.Set("X-Anonymous-Id", "anon-reader-state")
	followResp := httptest.NewRecorder()
	router.ServeHTTP(followResp, followReq)
	if followResp.Code != http.StatusOK {
		t.Fatalf("expected follow 200, got %d: %s", followResp.Code, followResp.Body.String())
	}
	if !strings.Contains(followResp.Body.String(), `"following":true`) {
		t.Fatalf("expected following response, got %s", followResp.Body.String())
	}

	detailReq := httptest.NewRequest(http.MethodGet, "/api/articles/"+article.Slug, nil)
	detailReq.Header.Set("X-Anonymous-Id", "anon-reader-state")
	detailResp := httptest.NewRecorder()
	router.ServeHTTP(detailResp, detailReq)
	if detailResp.Code != http.StatusOK {
		t.Fatalf("expected detail 200, got %d: %s", detailResp.Code, detailResp.Body.String())
	}
	var detail struct {
		UserBookmarked bool `json:"user_bookmarked"`
		AuthorFollowed bool `json:"author_followed"`
	}
	if err := json.Unmarshal(detailResp.Body.Bytes(), &detail); err != nil {
		t.Fatalf("decode detail: %v", err)
	}
	if !detail.UserBookmarked || !detail.AuthorFollowed {
		t.Fatalf("expected reader state on detail, got %+v", detail)
	}

	unbookmarkReq := httptest.NewRequest(http.MethodPost, "/api/articles/"+article.Slug+"/bookmark", strings.NewReader(`{"action":"unbookmark"}`))
	unbookmarkReq.Header.Set("Content-Type", "application/json")
	unbookmarkReq.Header.Set("X-Anonymous-Id", "anon-reader-state")
	unbookmarkResp := httptest.NewRecorder()
	router.ServeHTTP(unbookmarkResp, unbookmarkReq)
	if unbookmarkResp.Code != http.StatusOK || !strings.Contains(unbookmarkResp.Body.String(), `"bookmarked":false`) {
		t.Fatalf("expected unbookmark response, got %d: %s", unbookmarkResp.Code, unbookmarkResp.Body.String())
	}

	unfollowReq := httptest.NewRequest(http.MethodPost, "/api/authors/"+strconv.Itoa(int(article.AuthorID))+"/follow", strings.NewReader(`{"action":"unfollow"}`))
	unfollowReq.Header.Set("Content-Type", "application/json")
	unfollowReq.Header.Set("X-Anonymous-Id", "anon-reader-state")
	unfollowResp := httptest.NewRecorder()
	router.ServeHTTP(unfollowResp, unfollowReq)
	if unfollowResp.Code != http.StatusOK || !strings.Contains(unfollowResp.Body.String(), `"following":false`) {
		t.Fatalf("expected unfollow response, got %d: %s", unfollowResp.Code, unfollowResp.Body.String())
	}
}

func TestCommentEndpointCreatesApprovedComment(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:   "Commentable article",
		Content: "# body",
		Status:  "published",
	})

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()
	req := httptest.NewRequest(http.MethodPost, "/api/articles/"+article.Slug+"/comments", strings.NewReader(`{"author_name":"读者","content":"这篇文章很有启发，感谢分享。"}`))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Anonymous-Id", "anon-comment-1")
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusCreated {
		t.Fatalf("expected comment create 201, got %d: %s", resp.Code, resp.Body.String())
	}

	pageReq := httptest.NewRequest(http.MethodGet, "/articles/"+article.Slug, nil)
	pageResp := httptest.NewRecorder()
	router.ServeHTTP(pageResp, pageReq)
	if pageResp.Code != http.StatusOK {
		t.Fatalf("expected article page 200, got %d: %s", pageResp.Code, pageResp.Body.String())
	}
	if !strings.Contains(pageResp.Body.String(), "这篇文章很有启发") {
		t.Fatalf("expected approved comment on article page: %s", pageResp.Body.String())
	}
}

func TestCommentEndpointCreatesReplyComment(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:   "Replyable article",
		Content: "# body",
		Status:  "published",
	})

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()
	parentReq := httptest.NewRequest(http.MethodPost, "/api/articles/"+article.Slug+"/comments", strings.NewReader(`{"author_name":"读者甲","content":"这是一条主评论。"}`))
	parentReq.Header.Set("Content-Type", "application/json")
	parentReq.Header.Set("X-Anonymous-Id", "anon-reply-parent")
	parentResp := httptest.NewRecorder()
	router.ServeHTTP(parentResp, parentReq)
	if parentResp.Code != http.StatusCreated {
		t.Fatalf("expected parent comment 201, got %d: %s", parentResp.Code, parentResp.Body.String())
	}
	var parentPayload struct {
		ID uint `json:"id"`
	}
	if err := json.Unmarshal(parentResp.Body.Bytes(), &parentPayload); err != nil {
		t.Fatalf("decode parent response: %v", err)
	}

	replyBody := `{"author_name":"读者乙","content":"这是对主评论的回复。","parent_id":` + strconv.Itoa(int(parentPayload.ID)) + `}`
	replyReq := httptest.NewRequest(http.MethodPost, "/api/articles/"+article.Slug+"/comments", strings.NewReader(replyBody))
	replyReq.Header.Set("Content-Type", "application/json")
	replyReq.Header.Set("X-Anonymous-Id", "anon-reply-child")
	replyResp := httptest.NewRecorder()
	router.ServeHTTP(replyResp, replyReq)
	if replyResp.Code != http.StatusCreated {
		t.Fatalf("expected reply comment 201, got %d: %s", replyResp.Code, replyResp.Body.String())
	}
	if !strings.Contains(replyResp.Body.String(), `"parent_id":`+strconv.Itoa(int(parentPayload.ID))) {
		t.Fatalf("expected reply parent id in response: %s", replyResp.Body.String())
	}

	comments, err := app.Comments.ListApprovedByArticle(t.Context(), article.ID)
	if err != nil {
		t.Fatalf("list approved comments: %v", err)
	}
	if len(comments) != 1 || len(comments[0].Replies) != 1 {
		t.Fatalf("expected one parent with one reply, got %+v", comments)
	}
	if comments[0].Replies[0].Content != "这是对主评论的回复。" {
		t.Fatalf("unexpected reply content: %+v", comments[0].Replies[0])
	}
}

func TestCommentEndpointRejectsSensitiveContent(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:   "Strict comments",
		Content: "# body",
		Status:  "published",
	})

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()
	req := httptest.NewRequest(http.MethodPost, "/api/articles/"+article.Slug+"/comments", strings.NewReader(`{"author_name":"读者","content":"这条评论包含政治相关内容"}`))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Anonymous-Id", "anon-comment-2")
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusBadRequest {
		t.Fatalf("expected policy reject 400, got %d: %s", resp.Code, resp.Body.String())
	}
	if !strings.Contains(resp.Body.String(), "comment_policy_violation") {
		t.Fatalf("expected policy violation code, got: %s", resp.Body.String())
	}
}

func TestAdminCommentModeration(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:   "Moderated article",
		Content: "# body",
		Status:  "published",
	})
	comment, err := app.Comments.Create(t.Context(), service.CreateCommentInput{
		ArticleID:   article.ID,
		AuthorName:  "读者",
		Content:     "这是一条需要复核的评论。",
		AnonymousID: "anon-comment-3",
		IPAddress:   "127.0.0.1",
	})
	if err != nil {
		t.Fatalf("seed comment: %v", err)
	}

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()
	sessionID, csrfToken := testutil.CreateAdminSessionCookie(t, app)

	listReq := httptest.NewRequest(http.MethodGet, "/api/admin/comments", nil)
	listReq.AddCookie(&http.Cookie{Name: service.AdminSessionCookieName, Value: sessionID})
	listResp := httptest.NewRecorder()
	router.ServeHTTP(listResp, listReq)
	if listResp.Code != http.StatusOK {
		t.Fatalf("expected comments list 200, got %d: %s", listResp.Code, listResp.Body.String())
	}
	if !strings.Contains(listResp.Body.String(), "Moderated article") {
		t.Fatalf("expected article title in admin comments list: %s", listResp.Body.String())
	}

	body := strings.NewReader(`{"status":"rejected","rejection_reason":"不符合评论规范"}`)
	updateReq := httptest.NewRequest(http.MethodPut, "/api/admin/comments/"+strconv.Itoa(int(comment.ID))+"/status", body)
	updateReq.Header.Set("Content-Type", "application/json")
	updateReq.Header.Set("X-CSRF-Token", csrfToken)
	updateReq.AddCookie(&http.Cookie{Name: service.AdminSessionCookieName, Value: sessionID})
	updateResp := httptest.NewRecorder()
	router.ServeHTTP(updateResp, updateReq)
	if updateResp.Code != http.StatusOK {
		t.Fatalf("expected status update 200, got %d: %s", updateResp.Code, updateResp.Body.String())
	}

	pageReq := httptest.NewRequest(http.MethodGet, "/articles/"+article.Slug, nil)
	pageResp := httptest.NewRecorder()
	router.ServeHTTP(pageResp, pageReq)
	if strings.Contains(pageResp.Body.String(), "这是一条需要复核的评论") {
		t.Fatalf("rejected comment should not render on public page")
	}
}

func TestAdminMeReturnsCurrentSessionUser(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()
	sessionID, _ := testutil.CreateAdminSessionCookie(t, app)

	req := httptest.NewRequest(http.MethodGet, "/api/admin/me", nil)
	req.AddCookie(&http.Cookie{Name: service.AdminSessionCookieName, Value: sessionID})
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusOK {
		t.Fatalf("expected current user 200, got %d: %s", resp.Code, resp.Body.String())
	}

	var payload struct {
		User struct {
			Username string `json:"username"`
			Role     string `json:"role"`
		} `json:"user"`
	}
	if err := json.Unmarshal(resp.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if payload.User.Username != "admin" || payload.User.Role != "admin" {
		t.Fatalf("unexpected current user: %+v", payload.User)
	}
}

func TestAdminDashboardReturnsRealMetrics(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	category := testutil.SeedCategory(t, app, "Tech", "tech")
	now := time.Now().UTC()
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Published dashboard article",
		Content:     "# body",
		CategoryID:  &category.ID,
		Status:      "published",
		PublishedAt: &now,
	})
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:   "Draft dashboard article",
		Content: "# body",
		Status:  "draft",
	})
	if _, err := app.Comments.Create(t.Context(), service.CreateCommentInput{
		ArticleID:   article.ID,
		AuthorName:  "读者",
		Content:     "统计接口需要看到这条评论。",
		AnonymousID: "dashboard-comment",
		IPAddress:   "127.0.0.1",
	}); err != nil {
		t.Fatalf("seed comment: %v", err)
	}

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()
	sessionID, _ := testutil.CreateAdminSessionCookie(t, app)
	req := httptest.NewRequest(http.MethodGet, "/api/admin/dashboard", nil)
	req.AddCookie(&http.Cookie{Name: service.AdminSessionCookieName, Value: sessionID})
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusOK {
		t.Fatalf("expected dashboard 200, got %d: %s", resp.Code, resp.Body.String())
	}

	var payload struct {
		Stats struct {
			TotalArticles     int64 `json:"total_articles"`
			PublishedArticles int64 `json:"published_articles"`
			DraftArticles     int64 `json:"draft_articles"`
			TotalComments     int64 `json:"total_comments"`
			PendingComments   int64 `json:"pending_comments"`
			TotalLikes        int64 `json:"total_likes"`
			MonthlyViews      int64 `json:"monthly_views"`
			Followers         int64 `json:"followers"`
		} `json:"stats"`
		Activity []struct {
			Type        string `json:"type"`
			Title       string `json:"title"`
			Description string `json:"description"`
		} `json:"activity"`
		ViewsTrend []struct {
			Date  string `json:"date"`
			Views int64  `json:"views"`
		} `json:"views_trend"`
	}
	if err := json.Unmarshal(resp.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode dashboard response: %v", err)
	}
	if payload.Stats.TotalArticles != 2 || payload.Stats.PublishedArticles != 1 || payload.Stats.DraftArticles != 1 {
		t.Fatalf("unexpected article stats: %+v", payload.Stats)
	}
	if payload.Stats.TotalComments != 1 {
		t.Fatalf("expected one comment, got %+v", payload.Stats)
	}
	if len(payload.Activity) == 0 {
		t.Fatalf("expected recent activity")
	}
	if len(payload.ViewsTrend) != 30 {
		t.Fatalf("expected 30 trend points, got %d", len(payload.ViewsTrend))
	}
}

func TestAdminSettingsReadAndUpdateSiteConfig(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter, app.Comments).Router()
	sessionID, csrfToken := testutil.CreateAdminSessionCookie(t, app)

	getReq := httptest.NewRequest(http.MethodGet, "/api/admin/settings", nil)
	getReq.AddCookie(&http.Cookie{Name: service.AdminSessionCookieName, Value: sessionID})
	getResp := httptest.NewRecorder()
	router.ServeHTTP(getResp, getReq)
	if getResp.Code != http.StatusOK {
		t.Fatalf("expected settings 200, got %d: %s", getResp.Code, getResp.Body.String())
	}
	if strings.Contains(getResp.Body.String(), app.Config.Session.Secret) || strings.Contains(getResp.Body.String(), app.Config.Admin.InitPassword) {
		t.Fatalf("settings response leaked sensitive configuration: %s", getResp.Body.String())
	}

	updateReq := httptest.NewRequest(http.MethodPut, "/api/admin/settings", strings.NewReader(`{"site":{"title":"新的站点标题","description":"新的站点描述","base_url":"https://blog.example.test"}}`))
	updateReq.Header.Set("Content-Type", "application/json")
	updateReq.Header.Set("X-CSRF-Token", csrfToken)
	updateReq.AddCookie(&http.Cookie{Name: service.AdminSessionCookieName, Value: sessionID})
	updateResp := httptest.NewRecorder()
	router.ServeHTTP(updateResp, updateReq)
	if updateResp.Code != http.StatusOK {
		t.Fatalf("expected settings update 200, got %d: %s", updateResp.Code, updateResp.Body.String())
	}
	if app.Config.Site.Title != "新的站点标题" || app.Config.Site.BaseURL != "https://blog.example.test" {
		t.Fatalf("settings update did not apply to runtime config: %+v", app.Config.Site)
	}
}

func TestAdminWriteRequiresCSRF(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	sessionID, _ := testutil.CreateAdminSessionCookie(t, app)

	req := httptest.NewRequest(http.MethodPost, "/api/admin/categories", strings.NewReader(`{"name":"Tech","slug":"tech"}`))
	req.Header.Set("Content-Type", "application/json")
	req.AddCookie(&http.Cookie{Name: service.AdminSessionCookieName, Value: sessionID})
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusForbidden {
		t.Fatalf("expected csrf reject 403, got %d", resp.Code)
	}
}

func TestAdminUploadRejectsSVG(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	sessionID, csrfToken := testutil.CreateAdminSessionCookie(t, app)

	var buf bytes.Buffer
	writer := multipart.NewWriter(&buf)
	part, err := writer.CreateFormFile("file", "x.svg")
	if err != nil {
		t.Fatalf("create form file: %v", err)
	}
	if _, err := io.Copy(part, strings.NewReader("<svg></svg>")); err != nil {
		t.Fatalf("copy file body: %v", err)
	}
	if err := writer.Close(); err != nil {
		t.Fatalf("close multipart writer: %v", err)
	}

	req := httptest.NewRequest(http.MethodPost, "/api/admin/upload", &buf)
	req.Header.Set("Content-Type", writer.FormDataContentType())
	req.Header.Set("X-CSRF-Token", csrfToken)
	req.AddCookie(&http.Cookie{Name: service.AdminSessionCookieName, Value: sessionID})
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusUnsupportedMediaType {
		t.Fatalf("expected svg reject 415, got %d: %s", resp.Code, resp.Body.String())
	}
}

func TestAdminCreateArticleRejectsExternalCoverImage(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	sessionID, csrfToken := testutil.CreateAdminSessionCookie(t, app)

	reqBody := `{"title":"A","content":"# body","status":"draft","cover_image":"http://evil.example/x.jpg"}`
	req := httptest.NewRequest(http.MethodPost, "/api/admin/articles", strings.NewReader(reqBody))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-CSRF-Token", csrfToken)
	req.AddCookie(&http.Cookie{Name: service.AdminSessionCookieName, Value: sessionID})
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusBadRequest {
		t.Fatalf("expected cover image reject 400, got %d: %s", resp.Code, resp.Body.String())
	}
}

func TestLoginRateLimitTriggers(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	app.Config.RateLimit.LoginIPMaxAttempts = 1
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()

	requestBody := `{"username":"admin","password":"wrong"}`
	for i := 0; i < 2; i++ {
		req := httptest.NewRequest(http.MethodPost, "/api/admin/login", strings.NewReader(requestBody))
		req.Header.Set("Content-Type", "application/json")
		req.RemoteAddr = "127.0.0.1:1234"
		resp := httptest.NewRecorder()
		router.ServeHTTP(resp, req)
		if i == 0 && resp.Code != http.StatusUnauthorized {
			t.Fatalf("expected first login fail 401, got %d", resp.Code)
		}
		if i == 1 && resp.Code != http.StatusTooManyRequests {
			t.Fatalf("expected rate limit 429, got %d", resp.Code)
		}
	}
}

func TestPublicArticlesHideDraftAndFuturePublished(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	now := time.Now().UTC()
	future := now.Add(24 * time.Hour)
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Visible article",
		Content:     "# visible",
		Status:      "published",
		PublishedAt: &now,
	})
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:   "Draft article",
		Content: "# draft",
		Status:  "draft",
	})
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Future article",
		Content:     "# future",
		Status:      "published",
		PublishedAt: &future,
	})

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	req := httptest.NewRequest(http.MethodGet, "/api/articles?limit=10", nil)
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusOK {
		t.Fatalf("expected list 200, got %d: %s", resp.Code, resp.Body.String())
	}
	body := resp.Body.String()
	if !strings.Contains(body, "visible-article") {
		t.Fatalf("expected visible article in response: %s", body)
	}
	if strings.Contains(body, "draft-article") || strings.Contains(body, "future-article") {
		t.Fatalf("draft or future article leaked: %s", body)
	}
}

func TestPublicArticlesFilterByKeyword(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	now := time.Now().UTC()
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Quiet Interface Notes",
		Content:     "# visible",
		Status:      "published",
		PublishedAt: &now,
	})
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Database Operations",
		Content:     "# hidden",
		Status:      "published",
		PublishedAt: &now,
	})

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	req := httptest.NewRequest(http.MethodGet, "/api/articles?limit=10&keyword=interface", nil)
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusOK {
		t.Fatalf("expected list 200, got %d: %s", resp.Code, resp.Body.String())
	}
	body := resp.Body.String()
	if !strings.Contains(body, "quiet-interface-notes") {
		t.Fatalf("expected matching article in response: %s", body)
	}
	if strings.Contains(body, "database-operations") {
		t.Fatalf("non-matching article leaked: %s", body)
	}
}

func TestCategoryDeleteForbiddenWhenPublishedArticlesExist(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	category := testutil.SeedCategory(t, app, "Tech", "tech")
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:      "Published article",
		Content:    "# body",
		CategoryID: &category.ID,
		Status:     "published",
	})
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	sessionID, csrfToken := testutil.CreateAdminSessionCookie(t, app)

	req := httptest.NewRequest(http.MethodDelete, "/api/admin/categories/"+strconv.Itoa(int(category.ID)), nil)
	req.Header.Set("X-CSRF-Token", csrfToken)
	req.AddCookie(&http.Cookie{Name: service.AdminSessionCookieName, Value: sessionID})
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusConflict {
		t.Fatalf("expected delete conflict 409, got %d: %s", resp.Code, resp.Body.String())
	}
}

func TestLikeEndpointRateLimitTriggers(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	app.Config.RateLimit.LikeIPMaxRequests = 1
	app.Config.RateLimit.LikeArticleMaxActions = 10
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:   "Popular article",
		Content: "# body",
		Status:  "published",
	})

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	makeReq := func(anon string) *httptest.ResponseRecorder {
		req := httptest.NewRequest(http.MethodPost, "/api/articles/"+article.Slug+"/like", strings.NewReader(`{"action":"like"}`))
		req.Header.Set("Content-Type", "application/json")
		req.Header.Set("X-Anonymous-Id", anon)
		req.RemoteAddr = "127.0.0.1:4567"
		resp := httptest.NewRecorder()
		router.ServeHTTP(resp, req)
		return resp
	}

	first := makeReq("anon-like-1")
	if first.Code != http.StatusOK {
		t.Fatalf("expected first like 200, got %d: %s", first.Code, first.Body.String())
	}
	second := makeReq("anon-like-2")
	if second.Code != http.StatusTooManyRequests {
		t.Fatalf("expected rate limit 429, got %d: %s", second.Code, second.Body.String())
	}
}

func TestPublicPageSetsAnonymousCookieAndSecurityHeaders(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()

	req := httptest.NewRequest(http.MethodGet, "/", nil)
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusOK {
		t.Fatalf("expected home 200, got %d", resp.Code)
	}
	if !strings.Contains(resp.Header().Get("Set-Cookie"), "anonymous_id=") {
		t.Fatalf("expected anonymous cookie, got headers: %+v", resp.Header())
	}
	if !strings.Contains(resp.Header().Get("Set-Cookie"), "HttpOnly") {
		t.Fatalf("expected anonymous cookie to be HttpOnly, got headers: %+v", resp.Header())
	}
	if resp.Header().Get("X-Content-Type-Options") != "nosniff" {
		t.Fatalf("expected nosniff header, got %q", resp.Header().Get("X-Content-Type-Options"))
	}
	if !strings.Contains(resp.Header().Get("Content-Security-Policy"), "frame-ancestors 'none'") {
		t.Fatalf("expected security csp header, got %q", resp.Header().Get("Content-Security-Policy"))
	}
}

func TestCategoryPageHighlightsCurrentCategory(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	category := testutil.SeedCategory(t, app, "Editorial", "editorial")
	other := testutil.SeedCategory(t, app, "Tech", "tech")
	now := time.Now().UTC()
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Editorial story",
		Content:     "# editorial",
		CategoryID:  &category.ID,
		Status:      "published",
		PublishedAt: &now,
	})
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Tech story",
		Content:     "# tech",
		CategoryID:  &other.ID,
		Status:      "published",
		PublishedAt: &now,
	})

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	req := httptest.NewRequest(http.MethodGet, "/categories/editorial", nil)
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusOK {
		t.Fatalf("expected category page 200, got %d: %s", resp.Code, resp.Body.String())
	}
	body := resp.Body.String()
	if !strings.Contains(body, "Editorial story") {
		t.Fatalf("expected matching article in category page")
	}
	if strings.Contains(body, "Tech story") {
		t.Fatalf("category page should not list other categories' articles")
	}
}

func TestArticlePageRendersSanitizedHTMLWithoutEscaping(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	article := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:   "Rendered article",
		Content: "# Heading\n<script>alert(1)</script>\nParagraph",
		Status:  "published",
	})

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	req := httptest.NewRequest(http.MethodGet, "/articles/"+article.Slug, nil)
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusOK {
		t.Fatalf("expected article page 200, got %d: %s", resp.Code, resp.Body.String())
	}
	body := resp.Body.String()
	if !strings.Contains(body, "<h1>Heading</h1>") {
		t.Fatalf("expected rendered heading html, got %s", body)
	}
	if strings.Contains(body, "&lt;h1&gt;Heading&lt;/h1&gt;") {
		t.Fatalf("html should not be escaped: %s", body)
	}
	articleHTMLSection := body
	if start := strings.Index(body, `<div class="article-html">`); start >= 0 {
		articleHTMLSection = body[start:]
		if end := strings.Index(articleHTMLSection, `</div>`); end >= 0 {
			articleHTMLSection = articleHTMLSection[:end+len(`</div>`)]
		}
	}
	if strings.Contains(strings.ToLower(articleHTMLSection), "<script") {
		t.Fatalf("sanitized html should not include script tag in article body: %s", articleHTMLSection)
	}
}

func TestHomePageSplitsHeroAndShowsCategoryCounts(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	category := testutil.SeedCategory(t, app, "Design Theory", "design-theory")
	now := time.Now().UTC()
	pinnedAt := now.Add(-time.Hour)
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Pinned hero story",
		Content:     "# hero",
		CategoryID:  &category.ID,
		Status:      "published",
		IsPinned:    true,
		PublishedAt: &pinnedAt,
	})
	older := now.Add(-2 * time.Hour)
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Secondary story",
		Content:     "# body",
		CategoryID:  &category.ID,
		Status:      "published",
		PublishedAt: &older,
	})

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	req := httptest.NewRequest(http.MethodGet, "/", nil)
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusOK {
		t.Fatalf("expected home 200, got %d: %s", resp.Code, resp.Body.String())
	}
	body := resp.Body.String()
	if !strings.Contains(body, "Pinned hero story") {
		t.Fatalf("expected hero story title, got: %s", body)
	}
	if !strings.Contains(body, "Secondary story") {
		t.Fatalf("expected grid story title in body")
	}
	if !strings.Contains(body, "设计理论") {
		t.Fatalf("expected category name to appear in sidebar")
	}
	if !strings.Contains(body, `href="/categories/design-theory"`) {
		t.Fatalf("expected category link in sidebar")
	}
	if !strings.Contains(body, "分钟阅读") {
		t.Fatalf("expected read time annotation in body")
	}
}

func TestArticlePageRendersRelatedArticles(t *testing.T) {
	t.Parallel()

	app := testutil.NewApp(t)
	category := testutil.SeedCategory(t, app, "Tech", "tech")
	other := testutil.SeedCategory(t, app, "Lifestyle", "lifestyle")
	now := time.Now().UTC()
	current := testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Current article",
		Content:     "# current",
		CategoryID:  &category.ID,
		Status:      "published",
		PublishedAt: &now,
	})
	earlier := now.Add(-time.Hour)
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Sibling tech story",
		Content:     "# sibling",
		CategoryID:  &category.ID,
		Status:      "published",
		PublishedAt: &earlier,
	})
	testutil.SeedArticle(t, app, service.CreateArticleInput{
		Title:       "Unrelated lifestyle story",
		Content:     "# other",
		CategoryID:  &other.ID,
		Status:      "published",
		PublishedAt: &earlier,
	})

	router := NewHTTPHandler(app.Config, app.Auth, app.Categories, app.Articles, app.Likes, app.Uploads, app.Sessions, app.RateLimiter).Router()
	req := httptest.NewRequest(http.MethodGet, "/articles/"+current.Slug, nil)
	resp := httptest.NewRecorder()
	router.ServeHTTP(resp, req)
	if resp.Code != http.StatusOK {
		t.Fatalf("expected article page 200, got %d: %s", resp.Code, resp.Body.String())
	}
	body := resp.Body.String()
	if !strings.Contains(body, "相关文章") {
		t.Fatalf("expected Related Articles section header")
	}
	if !strings.Contains(body, "Sibling tech story") {
		t.Fatalf("expected sibling article in related list")
	}
	if strings.Contains(body, "Unrelated lifestyle story") {
		t.Fatalf("related list should be filtered by category, but unrelated article appeared")
	}
	if strings.Contains(body, ">Current article</a>") {
		t.Fatalf("related list must exclude the current article")
	}
}
