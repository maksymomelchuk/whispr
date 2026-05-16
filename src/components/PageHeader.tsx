import { cn } from "@/lib/utils";

interface Props {
  eyebrow?: string;
  title: string;
  subtitle?: string;
  trailing?: React.ReactNode;
  className?: string;
}

export function PageHeader({
  eyebrow,
  title,
  subtitle,
  trailing,
  className,
}: Props) {
  return (
    <header
      className={cn(
        "flex items-end justify-between gap-6 pb-7 border-b border-border/60",
        className,
      )}
    >
      <div className="flex flex-col gap-1.5 min-w-0">
        {eyebrow && (
          <span className="font-mono text-[10.5px] font-semibold uppercase tracking-[0.14em] text-muted-foreground/80">
            {eyebrow}
          </span>
        )}
        <h1 className="text-[22px] font-semibold leading-[1.15] text-foreground tracking-[-0.012em]">
          {title}
        </h1>
        {subtitle && (
          <p className="text-[13px] text-muted-foreground leading-snug max-w-prose">
            {subtitle}
          </p>
        )}
      </div>
      {trailing && <div className="shrink-0">{trailing}</div>}
    </header>
  );
}
