package handler

import (
	"strings"

	"blogWeb/internal/service"
)

type authorProfile struct {
	AvatarURL string
	Bio       string
}

type demoComment struct {
	Name         string
	AvatarURL    string
	RelativeTime string
	Content      string
}

var authorProfiles = map[string]authorProfile{
	"Elena Vance": {
		AvatarURL: "https://lh3.googleusercontent.com/aida-public/AB6AXuBKUsbsCZWwB3Y5gHlpgsLxHYQLmo58CcSGq0Mo2DNw2W-xrGvM7GFxtfh_GGSE3uRdNJlc45PDBnUCGrd8hoRu-0HLWgFbQzXMP-ZExVYbgc5-GbcOWXCeNDc1rOGccBZTxaVQsWFhG4z3Qn6AhyQKEfQSMJfJFLPjC7j9hms89tmrF0p2MR1d4ROrs4sfbgcNLw4BIgqlD1TNpUbtuV3FxBviwj_oTar08L4l3zbKkkgArWDpMSO9IPeAS6ebgHWadwlbHVjQq1w",
		Bio:       "林悦是一名设计策略师与文化评论者，长期关注技术、哲学与现代审美的交汇处。",
	},
	"Sarah Chen": {
		AvatarURL: "https://lh3.googleusercontent.com/aida-public/AB6AXuDhpHKomsu6tJO4VBl07esgfQDjerFJ_5Ekmlvrfad75dM45JXxDUqxBblqJ4wRF4p7gNCkdMWKMCgol7d15EwM5hVlFN84uYOzv4Y0Rm0loc9ZlS-ZdRNNWFq_fnBtweKZ823EDm2vsqiMAfQCgXFsa6gavqvZ9glk073qCEhox1FQ6iDnOsxvOXz1d8oLtZaD1IFlwL9LEcf0vUwgvJnakZIlj9lIhQz9n7udkOTCqjP3QxurLIyaTjFd6EfGKx5CsIrFzICDPQE",
		Bio:       "苏晴书写安静界面、架构系统，以及数字产品如何从物理空间中借鉴秩序。",
	},
	"Marcus Thorne": {
		AvatarURL: "https://lh3.googleusercontent.com/aida-public/AB6AXuCV-0FM_BiyxoFWSIvINulwnbGu0KrGJsYMCtvfkRsZNPEpMnxz_MRXNp0eUn1bqsAnJFTaLgmw03rPMmEtEGW1OSNLRdU-k2dhTv8HEj6Ew1nqbrvSM4ZVuKdaClyrRtJEcUVFwmokVMhgjZPemrZ1fw_2ySThUjb_o8VhEexp1PFEYG6xbfPidrZvy-E5CBXoRKhJZQlXOrxF2q6wAtCLBe78ubemWhXphcxL7mFRoYroBYBCPz6CksGhWQCZtgcXPy78_iQo6IM",
		Bio:       "陈默关注慢技术、注意力管理，以及刻意工作的组织成本。",
	},
	"Alex Rivera": {
		AvatarURL: "https://lh3.googleusercontent.com/aida-public/AB6AXuDE0ev-JB33hUlLQkaVuyMf7_37CN0aUjNFznTFc_8Fe1vq5YW2CgcRZ_olG3bWCTIHWgPJzGZ8wilwB1ZtkpzNOsP0H7feDbBPK5WykNPQfXNXt5VhkfGX67z4EGUhndyicLImn1Yk2TTkYIO-_DEJag3nMAUGnmGZQVnTOJ5MW73XPM5rJq7KnTlwVS4g1dDW7MbiCjEpdiiE1yGIgHRlesapsdQ1_f2jeTSY_d9c3dMvT2Ir2eHEyPvzPwNpF2gGoJVnbJBP0A0",
		Bio:       "陆远关注编辑系统、产品运营，以及维持现代出版团队节奏的日常习惯。",
	},
	"Casey Chen": {
		AvatarURL: "https://lh3.googleusercontent.com/aida-public/AB6AXuAdRurO9IgRv7Ok_taeu4z98Ov-PonJTaeaQ5GKA0sUSPePG_RNp9K9R76-JWZDeeETPCUX1WQMvn4-7oPnTFd7gRF9smqICKTS3YMKtbgsk1j2i4uD8HPMId-ngjFPRNyBj78-FYRqINfsOGmB9RrW04ka19m-FnU9-P5iOy81t5O908z6ZaU-e6dJcvuzWYPW6jPLtOEfb_OPF7VV2Ns7Jzmqd3fp-DL9Y092ntbzktQYA9HuCOJKy8jLr0ZRq9tgR70yWNUL1XQ",
		Bio:       "陈岚书写分布式团队、管理仪式，以及可持续的远程协作文化。",
	},
	"Jordan Smith": {
		AvatarURL: "https://lh3.googleusercontent.com/aida-public/AB6AXuDhmrJquSYtYV19zNRCJvrjM2NjPrs87IaNh9Q0MluV7v-DKYCsCIs8f9_ty7-uv39DJyn6ErBAjTMSgtCgzxbEswz91BOn9bveBhWeHPoxPRY3a89Y6SOr0Yhcf77tEUYmc1ybymEiDIx-zlbHae_iYtum4jUxTSdmK2i5mS6jNn8eBD1H-qQ2ncu7MRd09jT2DwRwaxOInFdhbU0aYeaMM7N-bPChdVDbMhOzROPqqWuvrcqm2dLGouWTpW2aG9MGqeO2PspidZg",
		Bio:       "周行关注前端工具、代码审美，以及开发者体验如何影响产品质量。",
	},
}

var defaultAuthorProfile = authorProfile{
	AvatarURL: "https://lh3.googleusercontent.com/aida-public/AB6AXuBKUsbsCZWwB3Y5gHlpgsLxHYQLmo58CcSGq0Mo2DNw2W-xrGvM7GFxtfh_GGSE3uRdNJlc45PDBnUCGrd8hoRu-0HLWgFbQzXMP-ZExVYbgc5-GbcOWXCeNDc1rOGccBZTxaVQsWFhG4z3Qn6AhyQKEfQSMJfJFLPjC7j9hms89tmrF0p2MR1d4ROrs4sfbgcNLw4BIgqlD1TNpUbtuV3FxBviwj_oTar08L4l3zbKkkgArWDpMSO9IPeAS6ebgHWadwlbHVjQq1w",
	Bio:       "长期探索设计、系统与有意识数字工作的作者。",
}

var categoryDisplayNames = map[string]string{
	"design-theory": "设计理论",
	"technology":    "技术",
	"architecture":  "架构",
	"engineering":   "工程",
	"editorial":     "编辑",
	"lifestyle":     "生活方式",
}

var authorDisplayNames = map[string]string{
	"admin":         "管理员",
	"Elena Vance":   "林悦",
	"Sarah Chen":    "苏晴",
	"Marcus Thorne": "陈默",
	"Alex Rivera":   "陆远",
	"Casey Chen":    "陈岚",
	"Jordan Smith":  "周行",
	"Sarah Jenkins": "苏晴",
}

func categoryName(slug, name string) string {
	if display, ok := categoryDisplayNames[strings.TrimSpace(slug)]; ok {
		return display
	}
	return strings.TrimSpace(name)
}

func authorName(name string) string {
	name = strings.TrimSpace(name)
	if display, ok := authorDisplayNames[name]; ok {
		return display
	}
	return name
}

func authorAvatar(name string) string {
	return authorProfileFor(name).AvatarURL
}

func authorBio(name string) string {
	return authorProfileFor(name).Bio
}

func authorProfileFor(name string) authorProfile {
	name = strings.TrimSpace(name)
	if profile, ok := authorProfiles[name]; ok {
		return profile
	}
	return defaultAuthorProfile
}

func demoCommentsForArticle(article *service.PublicArticleDetail) []demoComment {
	if article == nil {
		return nil
	}
	switch article.Slug {
	case "the-future-of-digital-curation-balancing-algorithms-and-human-intuition":
		return []demoComment{
			{
				Name:         "Sarah Jenkins",
				AvatarURL:    authorProfiles["Sarah Chen"].AvatarURL,
				RelativeTime: "2 小时前",
				Content:      "很有启发。我确实在自己的推荐流里感受到了信息茧房效应，也认同这些系统需要更多人的判断。",
			},
			{
				Name:         "Marcus Thorne",
				AvatarURL:    authorProfiles["Marcus Thorne"].AvatarURL,
				RelativeTime: "5 小时前",
				Content:      "你觉得小型社群会是算法疲劳的解法吗？小而精选的讨论组仍然是互联网上很有意思的地方。",
			},
		}
	case "minimalism-in-the-digital-workplace":
		return []demoComment{
			{
				Name:         "Casey Chen",
				AvatarURL:    authorProfiles["Casey Chen"].AvatarURL,
				RelativeTime: "昨天",
				Content:      "关于视觉安静能形成运营优势的部分很准确。我们简化远程交接界面后也看到了类似效果。",
			},
			{
				Name:         "Jordan Smith",
				AvatarURL:    authorProfiles["Jordan Smith"].AvatarURL,
				RelativeTime: "2 天前",
				Content:      "希望后续能展开这些原则如何迁移到移动端高密度工作流里，那里密度和清晰度经常互相拉扯。",
			},
		}
	default:
		return []demoComment{
			{
				Name:         "Elena Vance",
				AvatarURL:    authorProfiles["Elena Vance"].AvatarURL,
				RelativeTime: "1 天前",
				Content:      "这篇的编辑节奏很好。论证扎实，同时没有失去推进感，这比看起来更难。",
			},
			{
				Name:         "Alex Rivera",
				AvatarURL:    authorProfiles["Alex Rivera"].AvatarURL,
				RelativeTime: "3 天前",
				Content:      "这类文章适合慢慢读。它给读者留下了空间，让人在页面结束之后还能继续思考。",
			},
		}
	}
}

func articleHighlightsFor(article *service.PublicArticleDetail) []string {
	if article == nil {
		return nil
	}
	switch article.Slug {
	case "the-future-of-digital-curation-balancing-algorithms-and-human-intuition":
		return []string{
			"策展是一种智识上的严谨，也是一种编辑判断力。",
			"算法效率可能收窄品味，而不是拓展品味。",
			"强大的发布系统会把机器速度与人的判断结合起来。",
		}
	case "the-slow-tech-movement-reclaiming-focus":
		return []string{
			"慢技术是一种工作流纪律，不是怀旧。",
			"有意设置的摩擦可以提升专注，减少强迫式检查。",
			"团队在增加工具之前，需要先建立保护注意力的默认设置。",
		}
	default:
		return []string{
			"清晰界面首先减少认知阻力，其次才改善审美。",
			"当布局、写作和运营互相支撑时，编辑系统效果最好。",
			"长期优势不是新奇，而是在反复日常使用中仍然清晰。",
		}
	}
}
