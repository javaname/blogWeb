const categoryNameKeysBySlug = {
  'design-theory': 'data.categories.designTheory',
  technology: 'data.categories.technology',
  architecture: 'data.categories.architecture',
  engineering: 'data.categories.engineering',
  editorial: 'data.categories.editorial',
  lifestyle: 'data.categories.lifestyle',
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
