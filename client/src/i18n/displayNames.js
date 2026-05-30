const categoryNameKeysBySlug = {
  'design-theory': 'data.categories.designTheory',
  technology: 'data.categories.technology',
  architecture: 'data.categories.architecture',
  engineering: 'data.categories.engineering',
  editorial: 'data.categories.editorial',
  lifestyle: 'data.categories.lifestyle',
};

const userNameKeysByName = {
  admin: 'common.admin',
  'Elena Vance': 'data.authors.elenaVance',
  'Sarah Chen': 'data.authors.sarahChen',
  'Marcus Thorne': 'data.authors.marcusThorne',
  'Alex Rivera': 'data.authors.alexRivera',
  'Casey Chen': 'data.authors.caseyChen',
  'Jordan Smith': 'data.authors.jordanSmith',
  'Sarah Jenkins': 'data.authors.sarahJenkins',
};

function translatedOrFallback(t, key, fallback) {
  const translated = key ? t(key) : '';
  return translated && translated !== key ? translated : fallback;
}

export function categoryDisplayName(t, category) {
  if (!category) {
    return '';
  }
  const key = categoryNameKeysBySlug[category.slug];
  return translatedOrFallback(t, key, category.name || category.slug || '');
}

export function userDisplayName(t, user) {
  const name = typeof user === 'string' ? user : user?.username;
  if (!name) {
    return '';
  }
  return translatedOrFallback(t, userNameKeysByName[name], name);
}
