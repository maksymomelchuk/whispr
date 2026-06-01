export function OpenRouterLogo({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 48 48"
      className={className}
      aria-hidden="true"
    >
      <rect width="48" height="48" rx="9" fill="#7C3AED" />
      <circle cx="24" cy="13" r="4" fill="white" />
      <circle cx="13" cy="35" r="4" fill="white" />
      <circle cx="35" cy="35" r="4" fill="white" />
      <line
        x1="24"
        y1="17"
        x2="13"
        y2="31"
        stroke="white"
        strokeWidth="2.5"
        strokeLinecap="round"
      />
      <line
        x1="24"
        y1="17"
        x2="35"
        y2="31"
        stroke="white"
        strokeWidth="2.5"
        strokeLinecap="round"
      />
      <line
        x1="17"
        y1="35"
        x2="31"
        y2="35"
        stroke="white"
        strokeWidth="2.5"
        strokeLinecap="round"
      />
    </svg>
  );
}
