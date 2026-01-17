import { InlineCode } from "@/components/atoms/InlineCode/InlineCode";
import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";

export const metadata = { title: "Install" };

export default function InstallPage() {
  return (
    <SimplePageTemplate
      title="Install"
      lede="Chic is currently developed from source. These steps build the bootstrap compiler and the chic CLI."
    >
      <Prose>
        <p>
          Chic is an <strong>alpha</strong> project. Expect breaking changes. For the authoritative instructions, see
          the repo <InlineCode>README.md</InlineCode>.
        </p>
        <h2>Build the CLI</h2>
        <pre>
          <code>{`cargo build --bin chic\n./target/debug/chic --help`}</code>
        </pre>
        <h2>Run a build</h2>
        <pre>
          <code>{`./target/debug/chic build`}</code>
        </pre>
        <h2>Create a small project</h2>
        <pre>
          <code>{`./target/debug/chic init --template app-console --output ./hello\n./target/debug/chic build ./hello`}</code>
        </pre>
      </Prose>
    </SimplePageTemplate>
  );
}

