import { Prose } from "@/components/molecules/Prose/Prose";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";

export const metadata = { title: "Community" };

const REPO = "https://github.com/Chic-lang/Chic";

export default function CommunityPage() {
  return (
    <SimplePageTemplate title="Community" lede="Chic collaboration currently happens primarily on GitHub.">
      <Prose>
        <p>Community channels (chat/forums) may be added later; this page will link them once they exist.</p>
        <ul>
          <li>
            <a href={`${REPO}/issues`} target="_blank" rel="noreferrer">
              Browse issues
            </a>
          </li>
          <li>
            <a href={`${REPO}/blob/main/CONTRIBUTING.md`} target="_blank" rel="noreferrer">
              Contributing guide
            </a>
          </li>
          <li>
            <a href={`${REPO}/blob/main/SUPPORT.md`} target="_blank" rel="noreferrer">
              Support
            </a>
          </li>
        </ul>
      </Prose>
    </SimplePageTemplate>
  );
}

