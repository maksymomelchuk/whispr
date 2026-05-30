export function AssemblyAiLogo({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 32 32"
      fill="none"
      className={className}
      aria-hidden="true"
    >
      <rect width="32" height="32" rx="6" fill="#3B1F8C" />
      <path
        d="M16 7L23 23H9L16 7Z"
        stroke="white"
        strokeWidth="2"
        strokeLinejoin="round"
        fill="none"
      />
      <line x1="11.5" y1="18" x2="20.5" y2="18" stroke="white" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}
