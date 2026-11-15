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

function isSafeUrl(raw: string): boolean {
  const trimmed = raw.trim();
  if (trimmed.startsWith("/") || trimmed.startsWith("#") || trimmed.startsWith("./") || trimmed.startsWith("../")) {
    return true;
  }
  try {
    const parsed = new URL(trimmed, "https://example.invalid");
    return parsed.protocol === "http:" || parsed.protocol === "https:" || parsed.protocol === "mailto:";
  } catch {
    return false;
  }
}

function renderInline(text: string, depth = 0): ReactNode[] {
  if (depth > 4) return [text];
  const pattern =
    /(`[^`]+`|!\[[^\]]*\]\([^)\s]+\)|\[[^\]]+\]\([^)\s]+\)|<https?:\/\/[^>\s]+>|https?:\/\/[^\s<>()]+|\*\*\*[^*\n]+\*\*\*|\*\*[^*\n]+\*\*|(?<![\w])__[^_\n]+__(?![\w])|~~[^~\n]+~~|\*[^*\n]+\*|(?<![\w])_[^_\n]+_(?![\w])|\n)/g;
  const parts = text.split(pattern).filter((part) => part !== "");
  return parts.map((part, index) => {
    if (part === "\n") {
      return <br key={index} />;
    }
    if (part.startsWith("`") && part.endsWith("`") && part.length >= 2) {
      return <code key={index}>{part.slice(1, -1)}</code>;
    }
    const image = /^!\[([^\]]*)\]\(([^)\s]+)\)$/.exec(part);
    if (image) {
      if (!isSafeUrl(image[2])) return part;
      return (
        <img
          key={index}
          src={image[2]}
          alt={image[1]}
          className="markdown-image"
          loading="lazy"
        />
      );
    }
    const link = /^\[([^\]]+)\]\(([^)\s]+)\)$/.exec(part);
    if (link) {
      if (!isSafeUrl(link[2])) return renderInline(link[1], depth + 1);
      return (
        <a key={index} href={link[2]} target="_blank" rel="noreferrer">
          {renderInline(link[1], depth + 1)}
        </a>
      );
    }
    const angleUrl = /^<(https?:\/\/[^>\s]+)>$/.exec(part);
    if (angleUrl) {
      return (
        <a key={index} href={angleUrl[1]} target="_blank" rel="noreferrer">
          {angleUrl[1]}
        </a>
      );
    }
    if (/^https?:\/\/[^\s<>()]+$/.test(part)) {
      const cleaned = part.replace(/[.,;:!?。，；：！？]+$/, "");
      const trailing = part.slice(cleaned.length);
      return (
        <span key={index}>
          <a href={cleaned} target="_blank" rel="noreferrer">
            {cleaned}
          </a>
          {trailing}
        </span>
      );
    }
    if (part.startsWith("***") && part.endsWith("***") && part.length > 6) {
      return (
        <strong key={index}>
          <em>{renderInline(part.slice(3, -3), depth + 1)}</em>
        </strong>
      );
    }
    if (
      ((part.startsWith("**") && part.endsWith("**")) ||
        (part.startsWith("__") && part.endsWith("__"))) &&
      part.length > 4
    ) {
      return <strong key={index}>{renderInline(part.slice(2, -2), depth + 1)}</strong>;
    }
    if (part.startsWith("~~") && part.endsWith("~~") && part.length > 4) {
      return <del key={index}>{renderInline(part.slice(2, -2), depth + 1)}</del>;
    }
    if (
      ((part.startsWith("*") && part.endsWith("*")) ||
        (part.startsWith("_") && part.endsWith("_"))) &&
      part.length > 2
    ) {
      return <em key={index}>{renderInline(part.slice(1, -1), depth + 1)}</em>;
    }
    return part;
  });
}

function splitTableRow(line: string): string[] {
  const trimmed = line.trim().replace(/^\|/, "").replace(/\|$/, "");
  const cells: string[] = [];
  let current = "";
  for (let i = 0; i < trimmed.length; i += 1) {
    const ch = trimmed[i];
    if (ch === "\\" && trimmed[i + 1] === "|") {
      current += "|";
      i += 1;
      continue;
    }
    if (ch === "|") {
      cells.push(current.trim());
      current = "";
      continue;
    }
    current += ch;
  }
  cells.push(current.trim());
  return cells;
}

type TableAlign = "left" | "center" | "right" | undefined;

function parseTableAlignments(line: string): TableAlign[] | null {
  if (!line.includes("|")) return null;
  const cells = splitTableRow(line);
  if (cells.length === 0) return null;
  const aligns: TableAlign[] = [];
  for (const cell of cells) {
    if (!/^:?-{3,}:?$/.test(cell)) return null;
    if (cell.startsWith(":") && cell.endsWith(":")) aligns.push("center");
    else if (cell.endsWith(":")) aligns.push("right");
    else if (cell.startsWith(":")) aligns.push("left");
    else aligns.push(undefined);
  }
  return aligns;
}

const LIST_ITEM_RE = /^(\s*)(?:([-*+])|(\d+)[.)])\s+(.*)$/;
const TASK_PREFIX_RE = /^\[([ xX])\]\s+(.*)$/;

interface ParsedListItem {
  text: string;
  checked?: boolean;
  children: ReactNode[];
}

function parseList(lines: string[], startIndex: number): { node: ReactNode; nextIndex: number } {
  const firstMatch = LIST_ITEM_RE.exec(lines[startIndex]);
  if (!firstMatch) {
    return { node: null, nextIndex: startIndex };
  }
  const baseIndent = firstMatch[1].length;
  const isOrdered = Boolean(firstMatch[3]);
  const items: ParsedListItem[] = [];
  let index = startIndex;

  while (index < lines.length) {
    const line = lines[index];
    if (line.trim() === "") {
      let peek = index + 1;
      while (peek < lines.length && lines[peek].trim() === "") {
        peek += 1;
      }
      const nextMatch = peek < lines.length ? LIST_ITEM_RE.exec(lines[peek]) : null;
      if (!nextMatch || nextMatch[1].length < baseIndent) {
        break;
      }
      index = peek;
      continue;
    }

    const match = LIST_ITEM_RE.exec(line);
    if (match) {
      const indent = match[1].length;
      const itemOrdered = Boolean(match[3]);
      if (indent < baseIndent) {
        break;
      }
      if (indent > baseIndent) {
        if (items.length === 0) break;
        const nested = parseList(lines, index);
        if (nested.node) {
          items[items.length - 1].children.push(nested.node);
        }
        index = nested.nextIndex;
        continue;
      }
      if (itemOrdered !== isOrdered) {
        break;
      }
      const rawContent = match[4];
      const task = !isOrdered ? TASK_PREFIX_RE.exec(rawContent) : null;
      items.push({
        text: task ? task[2] : rawContent,
        checked: task ? task[1].toLowerCase() === "x" : undefined,
        children: [],
      });
      index += 1;
      continue;
    }

    // Continuation line indented deeper than the bullet marker
    const leadingSpaces = /^(\s*)/.exec(line)?.[1].length ?? 0;
    if (items.length > 0 && leadingSpaces > baseIndent && !/^(#{1,6})\s+/.test(line) && !/^```/.test(line)) {
      items[items.length - 1].text += ` ${line.trim()}`;
      index += 1;
      continue;
    }

    break;
  }

  const ListTag = isOrdered ? "ol" : "ul";
  const hasTasks = items.some((item) => item.checked !== undefined);
  const node = (
    <ListTag
      key={`${ListTag}-${startIndex}`}
      className={hasTasks ? "markdown-task-list" : undefined}
    >
      {items.map((item, itemIndex) =>
        item.checked !== undefined ? (
          <li key={itemIndex} className="markdown-task-item">
            <input
              type="checkbox"
              checked={item.checked}
              readOnly
              aria-label={item.checked ? "已完成" : "未完成"}
            />
            <div className="markdown-task-body">
              <span>{renderInline(item.text)}</span>
              {item.children}
            </div>
          </li>
        ) : (
          <li key={itemIndex}>
            {renderInline(item.text)}
            {item.children}
          </li>
        ),
      )}
    </ListTag>
  );
  return { node, nextIndex: index };
}

function isTableStart(lines: string[], index: number): boolean {
  if (index + 1 >= lines.length) return false;
  if (!lines[index].includes("|")) return false;
  return parseTableAlignments(lines[index + 1]) !== null;
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

    if (LIST_ITEM_RE.test(line)) {
      const { node, nextIndex } = parseList(lines, index);
      if (node) {
        blocks.push(node);
      }
      index = nextIndex > index ? nextIndex : index + 1;
      continue;
    }

    if (isTableStart(lines, index)) {
      const headers = splitTableRow(lines[index]);
      const aligns = parseTableAlignments(lines[index + 1]) ?? [];
      index += 2;
      const rows: string[][] = [];
      while (index < lines.length) {
        const rowLine = lines[index];
        if (
          rowLine.trim() === "" ||
          !rowLine.includes("|") ||
          /^```/.test(rowLine) ||
          /^(#{1,6})\s+/.test(rowLine) ||
          /^\s*(-{3,}|\*{3,}|_{3,})\s*$/.test(rowLine) ||
          /^\s*>\s?/.test(rowLine)
        ) {
          break;
        }
        rows.push(splitTableRow(rowLine));
        index += 1;
      }
      blocks.push(
        <div key={`table-${index}`} className="markdown-table-wrap">
          <table className="markdown-table">
            <thead>
              <tr>
                {headers.map((cell, cellIndex) => (
                  <th key={cellIndex} style={aligns[cellIndex] ? { textAlign: aligns[cellIndex] } : undefined}>
                    {renderInline(cell)}
                  </th>
                ))}
              </tr>
            </thead>
            {rows.length > 0 && (
              <tbody>
                {rows.map((row, rowIndex) => (
                  <tr key={rowIndex}>
                    {headers.map((_, cellIndex) => (
                      <td
                        key={cellIndex}
                        style={aligns[cellIndex] ? { textAlign: aligns[cellIndex] } : undefined}
                      >
                        {renderInline(row[cellIndex] ?? "")}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            )}
          </table>
        </div>,
      );
      continue;
    }

    const paragraphParts: string[] = [];
    while (index < lines.length) {
      const current = lines[index];
      const startsBlock =
        current.trim() === "" ||
        /^```/.test(current) ||
        /^(#{1,6})\s+/.test(current) ||
        /^\s*(-{3,}|\*{3,}|_{3,})\s*$/.test(current) ||
        /^\s*>\s?/.test(current) ||
        LIST_ITEM_RE.test(current) ||
        isTableStart(lines, index);
      if (startsBlock) {
        break;
      }
      const hardBreak = / {2,}$/.test(current) || /\\$/.test(current);
      const cleaned = current.replace(/\\$/, "").trim();
      paragraphParts.push(cleaned);
      index += 1;
      const next = lines[index];
      const nextContinues =
        next !== undefined &&
        next.trim() !== "" &&
        !/^```/.test(next) &&
        !/^(#{1,6})\s+/.test(next) &&
        !/^\s*(-{3,}|\*{3,}|_{3,})\s*$/.test(next) &&
        !/^\s*>\s?/.test(next) &&
        !LIST_ITEM_RE.test(next) &&
        !isTableStart(lines, index);
      if (nextContinues) {
        paragraphParts.push(hardBreak ? "\n" : " ");
      }
    }
    blocks.push(<p key={`p-${index}`}>{renderInline(paragraphParts.join(""))}</p>);
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
