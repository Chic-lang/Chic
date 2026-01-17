import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";

export const metadata = { title: "Governance" };

const REPO = "https://github.com/Chic-lang/Chic";

export default function GovernancePage() {
  return (
    <SimplePageTemplate title="Governance" lede="Chic is currently maintained as an early-stage, alpha project.">
      <Prose>
        <p>
          Governance is expected to evolve as the project grows. Today, the most reliable sources of truth are the repo
          and its docs/spec.
        </p>
        <ul>
          <li>
            <a href={`${REPO}/blob/main/README.md`} target="_blank" rel="noreferrer">
              README.md
            </a>
          </li>
          <li>
            <a href={`${REPO}/blob/main/docs/mission.md`} target="_blank" rel="noreferrer">
              docs/mission.md
            </a>
          </li>
          <li>
            <a href={`${REPO}/issues`} target="_blank" rel="noreferrer">
              Issues
            </a>
          </li>
        </ul>
      </Prose>
    </SimplePageTemplate>
  );
}

