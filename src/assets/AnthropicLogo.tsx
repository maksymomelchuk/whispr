export function AnthropicLogo({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      className={className}
      aria-hidden="true"
    >
      <rect width="24" height="24" rx="4.5" fill="#FFFFFF" />
      <rect
        x="0.5"
        y="0.5"
        width="23"
        height="23"
        rx="4"
        fill="none"
        stroke="#E2E8F0"
      />
      <g transform="translate(4 4) scale(0.6667)" fill="#000000">
        <path d="M17.3041 3.541h-3.6718l6.696 16.918H24Zm-10.6082 0L0 20.459h3.7442l1.3693-3.5527h7.0052l1.3693 3.5528h3.7442L10.5363 3.5409Zm-.3712 10.2232 2.2914-5.9456 2.2914 5.9456Z" />
      </g>
    </svg>
  );
}
