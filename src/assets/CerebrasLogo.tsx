export function CerebrasLogo({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 48 48"
      className={className}
      aria-hidden="true"
    >
      <rect width="48" height="48" rx="9" fill="#FF6000" />
      <polygon
        points="24,9 37.5,16.5 37.5,31.5 24,39 10.5,31.5 10.5,16.5"
        fill="none"
        stroke="white"
        strokeWidth="2.5"
      />
      <circle cx="24" cy="24" r="4.5" fill="white" />
      <circle cx="24" cy="13" r="2" fill="white" />
      <circle cx="33.5" cy="18.5" r="2" fill="white" />
      <circle cx="33.5" cy="29.5" r="2" fill="white" />
      <circle cx="24" cy="35" r="2" fill="white" />
      <circle cx="14.5" cy="29.5" r="2" fill="white" />
      <circle cx="14.5" cy="18.5" r="2" fill="white" />
    </svg>
  );
}
