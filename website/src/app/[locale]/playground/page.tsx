import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";

export const metadata = { title: "Playground" };

export default function PlaygroundPage() {
  return (
    <SimplePageTemplate title="Playground" lede="A web playground is not available yet.">
      <Prose>
        <p>
          Chic is still in alpha. A public playground will likely arrive later, after the language and runtime surface
          is more stable and the security model is well defined.
        </p>
        <p>For now, the best way to try Chic is locally via the CLI (see Install).</p>
      </Prose>
    </SimplePageTemplate>
  );
}

