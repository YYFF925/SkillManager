/**
 * 标签展示：柔和多色标签（沿用原型视觉基线）。
 *
 * 原型按演示标签的关键词指定颜色；正式产品对任意标签名做稳定散列，
 * 分配到同一套色调，使同一标签在各处颜色一致。
 */

const TAG_TONES = [
  "mint",
  "blue",
  "violet",
  "orange",
  "cyan",
  "rose",
] as const;

function toneOf(tag: string): string {
  let hash = 0;
  for (const char of tag) {
    hash = (hash * 31 + char.codePointAt(0)!) % 1000003;
  }
  return TAG_TONES[hash % TAG_TONES.length];
}

export function TagChip({ tag }: { tag: string }) {
  return (
    <span
      className={`tag-tone-${toneOf(tag)} inline-flex items-center justify-center rounded-full px-2 text-[11px] leading-[18px] font-medium whitespace-nowrap`}
    >
      {tag}
    </span>
  );
}
