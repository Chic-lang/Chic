import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";

export function Markdown({ markdown }: { markdown: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      rehypePlugins={[rehypeHighlight]}
      components={{
        a: ({ href, children, ...rest }) => {
          const url = href ?? "#";
          const isExternal = /^https?:\/\//.test(url);
          if (isExternal) {
            return (
              <a href={url} target="_blank" rel="noreferrer" {...rest}>
                {children}
              </a>
            );
          }
          return (
            <a href={url} {...rest}>
              {children}
            </a>
          );
        }
      }}
    >
      {markdown}
    </ReactMarkdown>
  );
}
