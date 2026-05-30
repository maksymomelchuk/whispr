export function OpenAiLogo({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 32 32"
      fill="none"
      className={className}
      aria-hidden="true"
    >
      <rect width="32" height="32" rx="6" fill="#0D0D0D" />
      <g stroke="white" strokeWidth="1.4" strokeLinecap="round" opacity="0.95">
        <ellipse cx="16" cy="16" rx="3.5" ry="7.5" />
        <ellipse cx="16" cy="16" rx="3.5" ry="7.5" transform="rotate(60 16 16)" />
        <ellipse cx="16" cy="16" rx="3.5" ry="7.5" transform="rotate(120 16 16)" />
      </g>
    </svg>
  );
}
