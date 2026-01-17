import Link from "next/link";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { Button } from "@/components/atoms/Button/Button";
import { Prose } from "@/components/molecules/Prose/Prose";

export default function NotFound() {
  return (
    <SimplePageTemplate title="Page not found" lede="That page doesn’t exist (or moved).">
      <Prose>
        <p>
          If you expected something here, please open an issue with the broken link and what you were trying to
          reach.
        </p>
        <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
          <Button href="/" variant="primary">
            Back to home
          </Button>
          <Link href="/docs">Go to docs</Link>
        </div>
      </Prose>
    </SimplePageTemplate>
  );
}

