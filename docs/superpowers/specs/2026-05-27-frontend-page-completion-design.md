# Frontend Page Completion Design

## Goal

Complete the visible frontend experience for both the public blog and the admin console. The work focuses on missing or broken surfaces: unreadable public-page copy, inert controls, thin admin settings, missing cover upload, and weak loading/empty/feedback states.

## Scope

- Public blog templates: home, category, article, shared header/sidebar/footer, and `public/assets/site.js`.
- Admin React app: shell, dashboard, posts, article editor, categories, comments, settings, shared API helpers, i18n messages, and styles.
- Verification: frontend build, existing i18n check, and targeted tests or script checks for new client behavior where the repo supports it.

## Public Blog Design

Public pages keep the existing Go template + Tailwind CDN architecture. The templates will be repaired in place so Chinese copy and template expressions render correctly. Header search becomes an actual overlay that lets readers enter a keyword and navigate to article search results. Buttons without backend support, such as subscribe, share, and bookmark, should provide clear feedback instead of silently doing nothing.

Article likes, comments, and reading progress remain in `site.js`. Existing messages in that script will be corrected, and failed network operations will show concise user-facing feedback.

## Admin Design

The admin console keeps its current React Router structure and custom CSS system. The article editor will expose image upload for covers using the existing `/api/admin/upload` endpoint. Lists and destructive operations gain confirmations and clear success/failure feedback. Empty and loading states should be consistent across posts, categories, comments, and dashboard sections.

Settings becomes a more complete operational page: site identity, publishing defaults, moderation policy, and system notes. If the backend does not expose persistence for a setting, the UI should show it as current policy or disabled read-only configuration rather than pretending it can save.

The shell notification and help controls become small panels with concrete content. Mobile layout should remain usable, especially table-like rows and editor/sidebar flows.

## Data Flow

- Admin requests continue through `apiRequest`, preserving cookies, CSRF tokens, and centralized 401/403 handling.
- Upload uses `uploadImage(file)` and writes the returned `url` into `cover_image`.
- Public search uses a query parameter on `/` and the existing public article list API or server-rendered filtering where available.
- Public feedback that does not require persistence stays client-side.

## Error Handling

Admin API failures continue to use centralized toast/error-message resolution. New local interactions should show inline status where the user initiated the action. Public-page errors use lightweight inline messages or alerts only when no better local surface exists.

## Verification

- Run `npm run check:i18n`.
- Run `npm run build`.
- Run relevant Go tests if template or handler behavior changes.
- Manually inspect `/`, `/categories/:slug`, `/articles/:slug`, and `/admin` flows when a local server is available.
