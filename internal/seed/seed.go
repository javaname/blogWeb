package seed

import (
	"context"
	"strings"
	"time"

	"blogWeb/internal/model"
	"blogWeb/internal/service"

	"golang.org/x/crypto/bcrypt"
	"gorm.io/gorm"
)

type categorySeed struct {
	Name string
	Slug string
}

type articleSeed struct {
	Title       string
	Author      string
	Role        string
	Category    string
	Status      string
	IsPinned    bool
	PublishedAt time.Time
	CoverImage  string
	Content     string
}

var categories = []categorySeed{
	{Name: "Design Theory", Slug: "design-theory"},
	{Name: "Technology", Slug: "technology"},
	{Name: "Architecture", Slug: "architecture"},
	{Name: "Engineering", Slug: "engineering"},
	{Name: "Editorial", Slug: "editorial"},
	{Name: "Lifestyle", Slug: "lifestyle"},
}

var articles = []articleSeed{
	{
		Title:       "The Future of Digital Curation: Balancing Algorithms and Human Intuition",
		Author:      "Elena Vance",
		Role:        "admin",
		Category:    "design-theory",
		Status:      "published",
		IsPinned:    true,
		PublishedAt: time.Date(2026, 5, 12, 9, 24, 0, 0, time.UTC),
		CoverImage:  "https://lh3.googleusercontent.com/aida-public/AB6AXuBToR6DGBPs21BtDc1Tavl13w_sArivt3ZJk08nT7gUbOqpfRQn27WghBEc7pDSAck6Kmwn5ZqFZOfchwv2NjQKjVKS_6xQBmRJkY8r2YNeGjhOeXVlOuOl44g0TdIhr2Zqzr0wUNJu0Qz66-PtvINhF5_oF587DN9W63Z2zLu_xdnAZLDWrieM6bayCtPe0iW6hlpopg7n22ejPausVKYt1HPX3GUjzhwOE5HdByUlGp_odzGcdlRuvZrYqCK-Ys1vwWMTUvUj7e8",
		Content: strings.TrimSpace(`
In an era where information is abundant but attention is scarce, the role of curation has never been more critical. As we navigate the vast digital landscape, we are constantly making choices about what to consume, what to share, and what to ignore. This process of selection and organization is the essence of curation.

But as algorithms become increasingly sophisticated, the line between human and machine-driven curation is blurring. Large language models and recommendation engines now shape our digital experiences in ways we are only beginning to understand. The question remains: can technology ever truly replicate the nuance of human taste?

> "True curation is not just about filtering; it is about storytelling. It's the ability to find a thread of meaning in a sea of noise and present it in a way that resonates with others."

## The Algorithmic Echo Chamber

Algorithms excel at efficiency. They can process millions of data points in milliseconds to find patterns and predict what we might like based on our past behavior. However, this efficiency comes at a cost. By reinforcing our existing preferences, algorithms can create echo chambers that limit our exposure to new ideas and diverse perspectives.

Human curators, on the other hand, bring a unique blend of intuition, empathy, and cultural context to the table. They can spot trends before they go viral, identify emerging voices, and make connections that an algorithm might miss.

As we look to the future, the most successful curation models will likely be those that leverage the strengths of both humans and machines. Technology can handle the heavy lifting of data processing, while humans provide the creative spark and ethical oversight needed to ensure a healthy digital ecosystem.
		`),
	},
	{
		Title:       "The Slow Tech Movement: Reclaiming Focus",
		Author:      "Marcus Thorne",
		Role:        "editor",
		Category:    "technology",
		Status:      "published",
		PublishedAt: time.Date(2026, 5, 9, 8, 10, 0, 0, time.UTC),
		CoverImage:  "https://lh3.googleusercontent.com/aida-public/AB6AXuDtsfLajyob_uhrDeddI_NB6wXq8I7HUkcuh1onDXITp1JJKUnuG5g2-1IwW0c-Q7rxfP_s73jBq_YavDkDMj50QZRn5Xtc5qQooMNTLMmqZMWaWoG3K8kgMwyfYQv9FW3nr2lR3ly3U3lIGe8KbuMx5tt1ynJQ8JNmXwG1YIqDv110E89aK9Z31I31kIkMNJTRO9RDfZGCMhAL0-T3ALv2RY4XhxHO_FXK9kOjb424ZzzOqZO9_Mqz3z2evULxgpbJm7ylnwiXKQ0",
		Content: strings.TrimSpace(`
Slow technology is not nostalgia disguised as taste. It is a practical response to the way hyper-optimized systems fragment attention. The more software minimizes every second of friction, the more likely it becomes that our workspaces optimize for reaction instead of reflection.

## Attention Is an Environment

Many teams treat focus as a personal discipline problem. In practice, it is usually a systems problem. Notification defaults, collaboration rituals, and interface density all change the amount of uninterrupted thought a person can protect during the day.

The best "slow" tools do not simply remove features. They establish clearer boundaries around urgency, context, and time. The value is not aesthetic restraint by itself. The value is reclaiming enough mental room to think deliberately.
		`),
	},
	{
		Title:       "Minimalism in the Digital Workplace",
		Author:      "Sarah Chen",
		Role:        "editor",
		Category:    "architecture",
		Status:      "published",
		PublishedAt: time.Date(2026, 5, 6, 10, 0, 0, 0, time.UTC),
		CoverImage:  "https://lh3.googleusercontent.com/aida-public/AB6AXuCSSDSaEPkQ7Euot3oI0vGESglzB8cRnjqm9TirGY0QidmFUMuMO3tSmaIqzz4rmi8TviXhlgNGqGMAdvREbFGcCuPsslo3W3H4deaP88ExVve_-Qb0IVPV3DNzjkKAExyJFXiFf9KfHA7TDQoIWECiaiSB2iY308_IdGvYX8Z_4i5YkTO1O4TlLH4qW-xDBSfIQ6gNIRtSeESMvt9HoN6kMCYUo7wjPYyWFLx-bqFySxweyt3zIvscAl33RtPLmt-R5cAqHNvvigk",
		Content: strings.TrimSpace(`
Minimalism becomes operationally useful when it makes repeated actions easier to scan, easier to compare, and easier to finish without hesitation. The workplace version of minimalism is not blankness. It is a disciplined refusal to let secondary decoration compete with primary workflow.

## Calm Surfaces, Faster Decisions

Quiet interfaces reduce the amount of interpretation required for everyday work. Teams move faster when the visual environment makes priority and hierarchy legible without explanation.

Design systems that lean into editorial restraint often create stronger software because they resist the urge to turn every feature into a performance. The result is not less capability. The result is more capability surviving contact with actual daily use.
		`),
	},
	{
		Title:       "Code as Poetry: The Aesthetics of Programming",
		Author:      "Jordan Smith",
		Role:        "editor",
		Category:    "engineering",
		Status:      "published",
		PublishedAt: time.Date(2026, 5, 2, 7, 45, 0, 0, time.UTC),
		CoverImage:  "https://lh3.googleusercontent.com/aida-public/AB6AXuAgbUXzVUESajmB-5x2z4ChfZh_ue_qKiYJ33KFhroPAVgsC4_4xMyel4bATPbU-ihaX66ZNmqOZulWtI1nF278BYXHojR3RyTdkQ_CVOaX7MwYlYNmYYbp5W5eQhPzOkh3vt1ILyvSo56YfJYTngYYnE_Pjvl-ZofKUE5MapffcMKbGKefIPdcKviAlltnawQ1iOBUwzbqohOX7wtJEVuiomzSorzsLO4pLtQLXEzC97VVG-V9udpobFv5wl1Rw3JE3YoeYAmbD-Q",
		Content: strings.TrimSpace(`
There is a reason experienced engineers talk about rhythm, line tension, and elegance when they describe good code. Those are aesthetic terms, but they are not decorative. They point to something practical: when structure is coherent, the next decision becomes easier.

## Beauty as Compression

Beautiful code is often just well-compressed intent. It removes noise without flattening meaning. The value is not that it looks impressive. The value is that it becomes easier to trust, easier to extend, and easier to explain to someone who did not write it.

Programming becomes more durable when teams care about both correctness and form. The two are not in conflict. Clarity is one of the strongest forms of correctness a codebase can have.
		`),
	},
	{
		Title:       "The Art of Long-form Digital Storytelling",
		Author:      "Alex Rivera",
		Role:        "admin",
		Category:    "editorial",
		Status:      "published",
		PublishedAt: time.Date(2026, 4, 29, 9, 0, 0, 0, time.UTC),
		CoverImage:  "https://lh3.googleusercontent.com/aida-public/AB6AXuA6oRT43X4-CFop51NkpJt25Z6f2iQAO09YxWvr5GAQ7LTelig-j27X4md1GyNWQ73HlQnEp_7Ec7t70X-ekUqsPY23cWle1fTt_KYD3IRxmr4bCQZ-RxOfxhNE7zR9AYBEUUQAlI4RJlT8W1Mjvq88i-RDVPo_yA2kcHbTf6genD9mGbzQbfQIdqW3AbTjTHxuMR6SkFpqZ5pxPfgVa7pUMiJIcm1aqpnustA3xJ75gUyHImeNufm5W-c4JngkpI-TfFQGS81GZWQ",
		Content: strings.TrimSpace(`
Long-form work still matters because it asks for a different contract between writer and reader. It replaces velocity with pacing, compression with sequence, and reaction with accumulation. The result is a format that can hold nuance without apology.

## Pacing Is the Product

Strong editorial systems treat pacing as part of interface design. Section breaks, pull quotes, supporting visuals, and typography all shape how an argument moves through time.

A well-designed reading surface helps the writer by protecting the reader from fatigue. That is why the best digital essays feel less like scrolling through content and more like moving through a carefully staged room.
		`),
	},
	{
		Title:       "Building a Remote Writing Culture",
		Author:      "Casey Chen",
		Role:        "editor",
		Category:    "lifestyle",
		Status:      "published",
		PublishedAt: time.Date(2026, 4, 24, 11, 30, 0, 0, time.UTC),
		CoverImage:  "https://lh3.googleusercontent.com/aida-public/AB6AXuBiI5s7MHYQZ6geb6lMltMXPG9XqgfM_z9GCPa_Gmv0XI_evY-1TXPVdFr8dUjILTCwQpSjQ6i8Rum6VfpXhMPJtU-DE7LVTLNsQSIj7PZ8SU-moKq1WtJ4sdsuxMw4P8oYiPp-xujgAZ8P3uuVFR7QzVyqUmvwoWBI64fXPb_qH-Rq3bPGE53IBYiITYX5A-Hfxlce002ISt_lbFK_HYNhruIjDIgymMORi41_4S65xTwXMw182jsUe2VuURbJ8YKaqcL0Et3ZbbI",
		Content: strings.TrimSpace(`
Remote writing teams do not become coherent through tooling alone. They become coherent through explicit editorial rituals: how drafts are introduced, how feedback is framed, how revision is scheduled, and how publishing decisions are made visible.

## Systems That Preserve Momentum

Distributed work breaks down when every conversation starts from scratch. The answer is not more meetings. The answer is a better record of intent and a clearer sequence of decisions.

The best remote cultures are usually the ones that make their editing process easy to inherit. Good notes, stable review rhythms, and quiet interfaces matter because they make coordination cheaper without flattening individual voice.
		`),
	},
	{
		Title:       "Why Tailwind CSS Is Dominating the Web",
		Author:      "Jordan Smith",
		Role:        "editor",
		Category:    "technology",
		Status:      "draft",
		PublishedAt: time.Date(2026, 5, 18, 8, 0, 0, 0, time.UTC),
		CoverImage:  "https://lh3.googleusercontent.com/aida-public/AB6AXuCV2UtPb9E67l6mxheasQnhHHKyrIOL1SIID-KQAW7tejVbUG-k7WuEHhu00K2POOlFtQOwnTP0AXg_0haqwg6uYhpkmsQv49mttZqPKOgZHtlghVyTyrUBzMXIlr2JYnCO1IguR2EbYmzNgXabOsb5sm_l2POyJQJAGRY7mlr5EZpYpzzFdMQcVmmdfTy-NVW-Cq1g0U5JavS8KSq-1CGLxhncHvAWb6ETUzEhC2iQAFoXItvMiZcyJmUsBFy6lgw0uMdHbn0O3b8",
		Content: strings.TrimSpace(`
This draft examines why utility-first workflows continue to win inside product teams that care about speed, consistency, and the cost of change.
		`),
	},
}

func EnsureDemoContent(ctx context.Context, db *gorm.DB, auth *service.AuthService, articleService *service.ArticleService, categoryService *service.CategoryService) error {
	var publishedCount int64
	if err := db.WithContext(ctx).Model(&model.Article{}).Where("status = ?", "published").Count(&publishedCount).Error; err != nil {
		return err
	}
	if publishedCount > 0 {
		return nil
	}

	if err := ensureUsers(ctx, db); err != nil {
		return err
	}
	if err := auth.EnsureInitialAdmin(ctx); err != nil {
		return err
	}

	categoryMap, err := ensureCategories(ctx, categoryService)
	if err != nil {
		return err
	}
	userMap, err := ensureUserMap(ctx, db)
	if err != nil {
		return err
	}

	for _, item := range articles {
		author := userMap[item.Author]
		var categoryID *uint
		if category, ok := categoryMap[item.Category]; ok {
			id := category.ID
			categoryID = &id
		}
		if _, err := articleService.Create(ctx, service.CreateArticleInput{
			Title:       item.Title,
			Content:     item.Content,
			CoverImage:  item.CoverImage,
			CategoryID:  categoryID,
			Status:      item.Status,
			IsPinned:    item.IsPinned,
			PublishedAt: ptrTime(item.PublishedAt),
			AuthorID:    author.ID,
		}); err != nil {
			return err
		}
	}
	return nil
}

func ensureUsers(ctx context.Context, db *gorm.DB) error {
	users := []struct {
		Username string
		Password string
		Role     string
	}{
		{Username: "Elena Vance", Password: "demo-password", Role: "admin"},
		{Username: "Sarah Chen", Password: "demo-password", Role: "editor"},
		{Username: "Marcus Thorne", Password: "demo-password", Role: "editor"},
		{Username: "Alex Rivera", Password: "demo-password", Role: "admin"},
		{Username: "Casey Chen", Password: "demo-password", Role: "editor"},
		{Username: "Jordan Smith", Password: "demo-password", Role: "editor"},
	}

	for _, item := range users {
		var count int64
		if err := db.WithContext(ctx).Model(&model.User{}).Where("username = ?", item.Username).Count(&count).Error; err != nil {
			return err
		}
		if count > 0 {
			continue
		}
		password, err := bcrypt.GenerateFromPassword([]byte(item.Password), bcrypt.DefaultCost)
		if err != nil {
			return err
		}
		if err := db.WithContext(ctx).Create(&model.User{
			Username: item.Username,
			Password: string(password),
			Role:     item.Role,
		}).Error; err != nil {
			return err
		}
	}
	return nil
}

func ensureCategories(ctx context.Context, categoriesService *service.CategoryService) (map[string]*model.Category, error) {
	result := make(map[string]*model.Category, len(categories))
	for index, item := range categories {
		category, err := categoriesService.Create(ctx, service.CreateCategoryInput{
			Name:      item.Name,
			Slug:      item.Slug,
			SortOrder: index,
		})
		if err != nil && !strings.Contains(strings.ToLower(err.Error()), "已存在") {
			return nil, err
		}
		if category == nil {
			category, err = categoriesService.GetBySlug(ctx, item.Slug)
			if err != nil {
				return nil, err
			}
		}
		result[item.Slug] = category
	}
	return result, nil
}

func ensureUserMap(ctx context.Context, db *gorm.DB) (map[string]model.User, error) {
	var users []model.User
	if err := db.WithContext(ctx).Find(&users).Error; err != nil {
		return nil, err
	}
	result := make(map[string]model.User, len(users))
	for _, user := range users {
		result[user.Username] = user
	}
	return result, nil
}

func ptrTime(value time.Time) *time.Time {
	return &value
}
