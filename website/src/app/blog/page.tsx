import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { BlogIndexTemplate } from "@/components/templates/BlogIndexTemplate/BlogIndexTemplate";

export const metadata = { title: "Blog" };

export default function BlogIndexPage() {
  return (
    <SimplePageTemplate title="Blog" lede="Updates and roadmap notes as Chic evolves.">
      <BlogIndexTemplate page={1} />
    </SimplePageTemplate>
  );
}
