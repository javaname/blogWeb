const icons = {
  add: (
    <>
      <path d="M12 5v14" />
      <path d="M5 12h14" />
    </>
  ),
  add_comment: (
    <>
      <path d="M5 5h14v10H8l-3 3V5z" />
      <path d="M12 8v4" />
      <path d="M10 10h4" />
    </>
  ),
  article: (
    <>
      <path d="M7 3h7l4 4v14H7V3z" />
      <path d="M14 3v5h4" />
      <path d="M9 12h6" />
      <path d="M9 16h6" />
    </>
  ),
  category: (
    <>
      <path d="M4 5h6v6H4z" />
      <path d="M14 5h6v6h-6z" />
      <path d="M4 15h6v4H4z" />
      <path d="M14 15h6v4h-6z" />
    </>
  ),
  chat_bubble: <path d="M5 5h14v10H9l-4 4V5z" />,
  chevron_left: <path d="M15 6l-6 6 6 6" />,
  chevron_right: <path d="M9 6l6 6-6 6" />,
  comment: <path d="M5 5h14v10H9l-4 4V5z" />,
  dashboard: (
    <>
      <path d="M4 4h7v8H4z" />
      <path d="M15 4h5v5h-5z" />
      <path d="M15 13h5v7h-5z" />
      <path d="M4 16h7v4H4z" />
    </>
  ),
  delete: (
    <>
      <path d="M5 7h14" />
      <path d="M9 7V5h6v2" />
      <path d="M8 7l1 13h6l1-13" />
      <path d="M10 11v5" />
      <path d="M14 11v5" />
    </>
  ),
  edit: (
    <>
      <path d="M5 19l4-1 9-9-3-3-9 9-1 4z" />
      <path d="M13 8l3 3" />
    </>
  ),
  expand_more: <path d="M6 9l6 6 6-6" />,
  group: (
    <>
      <path d="M9 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
      <path d="M3 20c.7-3.2 2.7-5 6-5s5.3 1.8 6 5" />
      <path d="M16 11a2.5 2.5 0 0 0 0-5" />
      <path d="M18 20c-.3-1.9-1.2-3.3-2.7-4.1" />
    </>
  ),
  help: (
    <>
      <path d="M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z" />
      <path d="M9.7 9a2.3 2.3 0 1 1 3.8 1.7c-.8.6-1.5 1.1-1.5 2.3" />
      <path d="M12 17h.01" />
    </>
  ),
  image: (
    <>
      <path d="M5 5h14v14H5z" />
      <path d="M8 15l3-3 2 2 2-3 3 4" />
      <path d="M9 9h.01" />
    </>
  ),
  moon: (
    <>
      <path d="M21 13a8 8 0 0 1-10-10 9 9 0 1 0 10 10z" />
    </>
  ),
  notifications: (
    <>
      <path d="M18 10a6 6 0 1 0-12 0c0 3-1.5 4.5-2.5 5.5h17C19.5 14.5 18 13 18 10z" />
      <path d="M10 19a2 2 0 0 0 4 0" />
    </>
  ),
  sun: (
    <>
      <path d="M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z" />
      <path d="M12 2v2" />
      <path d="M12 20v2" />
      <path d="M4.93 4.93l1.41 1.41" />
      <path d="M17.66 17.66l1.41 1.41" />
      <path d="M2 12h2" />
      <path d="M20 12h2" />
      <path d="M4.93 19.07l1.41-1.41" />
      <path d="M17.66 6.34l1.41-1.41" />
    </>
  ),
  person_add: (
    <>
      <path d="M9 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
      <path d="M3 20c.7-3.2 2.7-5 6-5 1.2 0 2.3.2 3.2.7" />
      <path d="M17 12v6" />
      <path d="M14 15h6" />
    </>
  ),
  publish: (
    <>
      <path d="M12 16V5" />
      <path d="M8 9l4-4 4 4" />
      <path d="M5 19h14" />
    </>
  ),
  search: (
    <>
      <path d="M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14z" />
      <path d="M16 16l4 4" />
    </>
  ),
  settings: (
    <>
      <path d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
      <path d="M19 12a7.2 7.2 0 0 0-.1-1l2-1.5-2-3.4-2.4 1a7.4 7.4 0 0 0-1.7-1L14.5 3h-5l-.3 3.1c-.6.3-1.2.6-1.7 1l-2.4-1-2 3.4 2 1.5a7.2 7.2 0 0 0 0 2l-2 1.5 2 3.4 2.4-1c.5.4 1.1.7 1.7 1l.3 3.1h5l.3-3.1c.6-.3 1.2-.6 1.7-1l2.4 1 2-3.4-2-1.5c.1-.3.1-.7.1-1z" />
    </>
  ),
  trending_down: (
    <>
      <path d="M4 7l6 6 4-4 6 6" />
      <path d="M20 10v5h-5" />
    </>
  ),
  trending_up: (
    <>
      <path d="M4 17l6-6 4 4 6-6" />
      <path d="M15 9h5v5" />
    </>
  ),
  tune: (
    <>
      <path d="M4 7h10" />
      <path d="M18 7h2" />
      <path d="M16 5v4" />
      <path d="M4 12h3" />
      <path d="M11 12h9" />
      <path d="M9 10v4" />
      <path d="M4 17h12" />
      <path d="M20 17h0" />
      <path d="M18 15v4" />
    </>
  ),
  update: (
    <>
      <path d="M20 6v5h-5" />
      <path d="M4 18v-5h5" />
      <path d="M18.5 9A7 7 0 0 0 6.6 6.6L4 9" />
      <path d="M5.5 15A7 7 0 0 0 17.4 17.4L20 15" />
    </>
  ),
  visibility: (
    <>
      <path d="M3 12s3.5-6 9-6 9 6 9 6-3.5 6-9 6-9-6-9-6z" />
      <path d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
    </>
  ),
};

export default function AdminIcon({ name, className = '', title }) {
  const icon = icons[name] || icons.help;
  const classes = ['admin-icon', className].filter(Boolean).join(' ');

  return (
    <svg
      aria-hidden={title ? undefined : 'true'}
      className={classes}
      fill="none"
      focusable="false"
      role={title ? 'img' : undefined}
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="2"
      viewBox="0 0 24 24"
    >
      {title ? <title>{title}</title> : null}
      {icon}
    </svg>
  );
}
