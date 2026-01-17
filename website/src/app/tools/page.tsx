import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";

export const metadata = { title: "Tools" };

const REPO = "https://github.com/Chic-lang/Chic";

export default function ToolsPage() {
  return (
    <SimplePageTemplate title="Tools" lede="Tooling is a core part of Chic: clear diagnostics and automation-friendly workflows.">
      <Prose>
        <h2>chic CLI</h2>
        <p>
          The <code>chic</code> executable is the primary interface today: build, test, run, and project scaffolding.
        </p>
        <h2>VS Code extension</h2>
        <p>
          The repo contains a VS Code extension under <code>chic-vscode/</code> (syntax + LSP client).
        </p>
        <p>
          Source:{" "}
          <a href={`${REPO}/tree/main/chic-vscode`} target="_blank" rel="noreferrer">
            chic-vscode/
          </a>
        </p>
      </Prose>
    </SimplePageTemplate>
  );
}

