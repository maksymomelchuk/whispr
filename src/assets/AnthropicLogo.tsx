export function AnthropicLogo({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 32 32"
      fill="none"
      className={className}
      aria-hidden="true"
    >
      <rect width="32" height="32" rx="6" fill="#D97757" />
      <path
        d="M19.2 8h-3.1L11 24h2.9l1.1-3h6.1l1.1 3H25L19.2 8zm-3.4 10 2.3-6.2 2.3 6.2h-4.6z"
        fill="white"
      />
    </svg>
  );
}
