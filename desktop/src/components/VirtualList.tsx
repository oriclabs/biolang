import { useMemo, useRef, useState, type ReactNode, type UIEvent } from "react";

/**
 * Minimal fixed-row virtualizer — no external dependency.
 *
 * Used for Explorer trees, Jobs, and Search when lists grow large enough that
 * mounting every row slows the main thread.
 */
export function VirtualList({
  count,
  itemHeight,
  height,
  overscan = 6,
  renderItem,
  className = "",
}: {
  count: number;
  itemHeight: number;
  height: number;
  overscan?: number;
  renderItem: (index: number) => ReactNode;
  className?: string;
}) {
  const [scrollTop, setScrollTop] = useState(0);
  const ref = useRef<HTMLDivElement>(null);
  const total = count * itemHeight;
  const start = Math.max(0, Math.floor(scrollTop / itemHeight) - overscan);
  const visible = Math.ceil(height / itemHeight) + overscan * 2;
  const end = Math.min(count, start + visible);
  const items = useMemo(() => {
    const next: ReactNode[] = [];
    for (let index = start; index < end; index += 1) {
      next.push(
        <div
          key={index}
          className="virtual-list-row"
          style={{
            position: "absolute",
            top: index * itemHeight,
            left: 0,
            right: 0,
            height: itemHeight,
          }}
        >
          {renderItem(index)}
        </div>,
      );
    }
    return next;
  }, [end, itemHeight, renderItem, start]);

  const onScroll = (event: UIEvent<HTMLDivElement>) => {
    setScrollTop(event.currentTarget.scrollTop);
  };

  return (
    <div
      ref={ref}
      className={`virtual-list ${className}`}
      style={{ height, overflow: "auto", position: "relative" }}
      onScroll={onScroll}
    >
      <div style={{ height: total, position: "relative" }}>{items}</div>
    </div>
  );
}
