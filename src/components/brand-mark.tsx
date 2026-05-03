import iconUrl from "@/assets/veyra-icon.png";
import { cn } from "@/lib/utils";

export function BrandMark({
  className,
  imageClassName,
}: {
  className?: string;
  imageClassName?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex shrink-0 items-center justify-center overflow-hidden rounded-xl bg-[#071018] shadow-[inset_0_0_0_1px_rgb(255_255_255_/_0.12),0_8px_22px_rgb(10_20_32_/_0.24)]",
        className,
      )}
      aria-hidden="true"
    >
      <img
        src={iconUrl}
        alt=""
        className={cn("h-full w-full object-cover", imageClassName)}
        draggable={false}
      />
    </span>
  );
}
