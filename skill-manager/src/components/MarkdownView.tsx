import type { ReactNode } from "react";

/**
 * 只读 Markdown（轻量标记语言）正文视图。
 *
 * 覆盖技能正文常见语法：frontmatter、标题、段落、列表、引用、分隔线、围栏代码与常用行内格式。
 * 不接入 HTML，不执行脚本；未知语法按普通文本保留。
 */

interface FrontmatterEntry {
  key: string;
  value: string;
}

function splitFrontmatter(text: string): { frontmatter: FrontmatterEntry[]; body: string } {
  const lines = text.replace(/\r\n?/g, "\n").split("\n");
  if (lines[0]?.trim() !== "---") {
    return { frontmatter: [], body: text };
  }
  const end = lines.findIndex((line, index) => index > 0 && line.trim() === "---");
  if (end < 0) {
    return { frontmatter: [], body: text };
  }
  const frontmatter = lines
    .slice(1, end)
    .map((line) => {
      const separator = line.indexOf(":");
      if (separator < 0) {
        return { key: line.trim(), value: "" };
      }
      return {
        key: line.slice(0, separator).trim(),
        value: line.slice(separator + 1).trim(),
      };
    })
    .filter((entry) => entry.key !== "");
  return { frontmatter, body: lines.slice(end + 1).join("\n").replace(/^\n+/, "") };
}

function renderInline(text: string): ReactNode[] {
  const pattern = /(`[^`]+`|\[[^\]]+\]\([^)]+\)|\*\*[^*]+\*\*|\*[^*]+\*)/g;
  return text
    .split(pattern)
    .filter((part) => part !== "")
    .map((part, index) => {
      if (part.startsWith("`") && part.endsWith("`")) {
        return <code key={index}>{part.slice(1, -1)}</code>;
      }
      const link = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(part);
      if (link) {
        return (
          <a key={index} href={link[2]} target="_blank" rel="noreferrer">
            {link[1]}
          </a>
        );
      }
      if (part.startsWith("**") && part.endsWith("**")) {
        return <strong key={index}>{part.slice(2, -2)}</strong>;
      }
      if (part.startsWith("*") && part.endsWith("*")) {
        return <em key={index}>{part.slice(1, -1)}</em>;
      }
      return part;
    });
}

function parseBlocks(lines: string[]): ReactNode[] {
  const blocks: ReactNode[] = [];
  let index = 0;

  while (index < lines.length) {
    const line = lines[index];
    if (line.trim() === "") {
      index += 1;
      continue;
    }

    const fence = /^```(.*)$/.exec(line);
    if (fence) {
      const language = fence[1].trim();
      const code: string[] = [];
      index += 1;
      while (index < lines.length && !lines[index].startsWith("```")) {
        code.push(lines[index]);
        index += 1;
      }
      if (index < lines.length) {
        index += 1;
      }
      blocks.push(
        <pre key={`code-${index}`} className="markdown-code-block">
          {language && <span className="markdown-code-language">{language}</span>}
          <code>{code.join("\n")}</code>
        </pre>,
      );
      continue;
    }

    const heading = /^(#{1,6})\s+(.+)$/.exec(line);
    if (heading) {
      const level = Math.min(heading[1].length, 4);
      const Tag = `h${level}` as "h1" | "h2" | "h3" | "h4";
      blocks.push(<Tag key={`heading-${index}`}>{renderInline(heading[2])}</Tag>);
      index += 1;
      continue;
    }

    if (/^\s*(-{3,}|\*{3,}|_{3,})\s*$/.test(line)) {
      blocks.push(<hr key={`rule-${index}`} />);
      index += 1;
      continue;
    }

    if (/^\s*>\s?/.test(line)) {
      const quote: string[] = [];
      while (index < lines.length && /^\s*>\s?/.test(lines[index])) {
        quote.push(lines[index].replace(/^\s*>\s?/, ""));
        index += 1;
      }
      blocks.push(
        <blockquote key={`quote-${index}`}>
          {parseBlocks(quote)}
        </blockquote>,
      );
      continue;
    }

    const unordered = /^\s*[-*+]\s+(.+)$/;
    if (unordered.test(line)) {
      const items: string[] = [];
      while (index < lines.length) {
        const match = unordered.exec(lines[index]);
        if (!match) {
          break;
        }
        items.push(match[1]);
        index += 1;
      }
      blocks.push(
        <ul key={`ul-${index}`}>
          {items.map((item, itemIndex) => (
            <li key={itemIndex}>{renderInline(item)}</li>
          ))}
        </ul>,
      );
      continue;
    }

    const ordered = /^\s*\d+[.)]\s+(.+)$/;
    if (ordered.test(line)) {
      const items: string[] = [];
      while (index < lines.length) {
        const match = ordered.exec(lines[index]);
        if (!match) {
          break;
        }
        items.push(match[1]);
        index += 1;
      }
      blocks.push(
        <ol key={`ol-${index}`}>
          {items.map((item, itemIndex) => (
            <li key={itemIndex}>{renderInline(item)}</li>
          ))}
        </ol>,
      );
      continue;
    }

    const paragraph: string[] = [];
    while (index < lines.length) {
      const current = lines[index];
      const startsBlock =
        current.trim() === "" ||
        /^```/.test(current) ||
        /^(#{1,6})\s+/.test(current) ||
        /^\s*(-{3,}|\*{3,}|_{3,})\s*$/.test(current) ||
        /^\s*>\s?/.test(current) ||
        unordered.test(current) ||
        ordered.test(current);
      if (startsBlock) {
        break;
      }
      paragraph.push(current.trim());
      index += 1;
    }
    blocks.push(<p key={`p-${index}`}>{renderInline(paragraph.join(" "))}</p>);
  }

  return blocks;
}

export function MarkdownView({ text, className = "" }: { text: string; className?: string }) {
  const { frontmatter, body } = splitFrontmatter(text);
  const blocks = parseBlocks(body.replace(/\r\n?/g, "\n").split("\n"));
  return (
    <div className={`markdown-body ${className}`}>
      {frontmatter.length > 0 && (
        <dl className="markdown-frontmatter">
          {frontmatter.map((entry) => (
            <div key={entry.key}>
              <dt>{entry.key}</dt>
              <dd>{entry.value}</dd>
            </div>
          ))}
        </dl>
      )}
      {blocks}
    </div>
  );
}
