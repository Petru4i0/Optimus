import { ReactNode } from "react";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

type StatusPillProps = {
  children: ReactNode;
  className?: string;
};

function cn(...inputs: (string | undefined | false)[]) {
  return twMerge(clsx(inputs));
}

export default function StatusPill({ children, className }: StatusPillProps) {
  return <div className={cn("pill", className)}>{children}</div>;
}
