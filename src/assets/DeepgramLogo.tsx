export function DeepgramLogo({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 32 32"
      fill="none"
      className={className}
      aria-hidden="true"
    >
      <rect width="32" height="32" rx="6" fill="#101827" />
      <path
        d="M7 16c0-4.97 4.03-9 9-9s9 4.03 9 9-4.03 9-9 9"
        stroke="#13EF93"
        strokeWidth="2.5"
        strokeLinecap="round"
      />
      <path
        d="M7 16c0-2.76 2.24-5 5-5s5 2.24 5 5-2.24 5-5 5"
        stroke="#13EF93"
        strokeWidth="2.5"
        strokeLinecap="round"
      />
      <circle cx="12" cy="16" r="2" fill="#13EF93" />
    </svg>
  );
}
