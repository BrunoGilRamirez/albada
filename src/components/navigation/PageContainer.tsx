import type { ReactNode } from "react";

interface PageContainerProps {
  children: ReactNode;
  variant?: "fluid" | "reading";
  className?: string;
}

function PageContainer({ children, variant = "fluid", className }: PageContainerProps) {
  const widthClass =
    variant === "reading" ? "w-full max-w-4xl mx-auto" : "w-full max-w-[2400px] mx-auto";
  return (
    <div className={`${widthClass} px-4 py-6 md:px-6 xl:px-8 3xl:px-12 ${className ?? ""}`}>
      {children}
    </div>
  );
}

export default PageContainer;
