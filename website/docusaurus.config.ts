import { themes as prismThemes } from "prism-react-renderer";
import type { Config } from "@docusaurus/types";
import type * as Preset from "@docusaurus/preset-classic";

const umamiWebsiteId = process.env.UMAMI_WEBSITE_ID?.trim();
/** Set UMAMI_DIRECT=1 for local preview against umami.chtnnhfoundation.org (no Worker). */
const umamiDirect = process.env.UMAMI_DIRECT === "1";
const umamiOrigin =
  process.env.UMAMI_ORIGIN?.trim() || "https://umami.chtnnhfoundation.org";
const umamiProxyPrefix = "/stats";

const umamiScript = umamiWebsiteId
  ? umamiDirect
    ? {
        src: `${umamiOrigin}/script.js`,
        defer: true as const,
        "data-website-id": umamiWebsiteId,
        "data-domains": "gg.chtnnhfoundation.org",
      }
    : {
        src: `${umamiProxyPrefix}/script.js`,
        defer: true as const,
        "data-website-id": umamiWebsiteId,
        "data-host-url": umamiProxyPrefix,
        "data-domains": "gg.chtnnhfoundation.org",
      }
  : null;

const config: Config = {
  title: "git-gist",
  tagline: "Run git across all child repositories",
  favicon: "img/logo.svg",

  future: {
    v4: true,
  },

  url: "https://gg.chtnnhfoundation.org",
  baseUrl: "/",

  organizationName: "chtnnh",
  projectName: "git-gist",

  onBrokenLinks: "throw",

  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },

  // First-party /stats proxy (Cloudflare Worker). Injected when UMAMI_WEBSITE_ID is set.
  scripts: umamiScript ? [umamiScript] : [],

  presets: [
    [
      "classic",
      {
        docs: {
          routeBasePath: "/",
          sidebarPath: "./sidebars.ts",
          editUrl: "https://github.com/chtnnh/git-gist/tree/main/website/",
          // Released snapshots are default (first entry in versions.json).
          // website/docs/ is HEAD — visitors must switch explicitly via the dropdown.
          includeCurrentVersion: true,
          versions: {
            current: {
              label: "HEAD",
              path: "head",
              banner: "unreleased",
            },
          },
        },
        blog: false,
        theme: {
          customCss: "./src/css/custom.css",
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    image: "img/logo.svg",
    colorMode: {
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: "git-gist",
      logo: {
        alt: "git-gist",
        src: "img/logo.svg",
      },
      items: [
        {
          type: "docSidebar",
          sidebarId: "docsSidebar",
          position: "left",
          label: "Docs",
        },
        {
          type: "docsVersionDropdown",
          position: "left",
          dropdownActiveClassDisabled: true,
        },
        {
          href: "https://github.com/chtnnh/git-gist",
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Docs",
          items: [
            { label: "Install", to: "/install" },
            { label: "Quick start", to: "/quickstart" },
            { label: "Configuration", to: "/config" },
          ],
        },
        {
          title: "Project",
          items: [
            {
              label: "GitHub",
              href: "https://github.com/chtnnh/git-gist",
            },
            {
              label: "Releases",
              href: "https://github.com/chtnnh/git-gist/releases",
            },
            {
              label: "Changelog",
              href: "https://github.com/chtnnh/git-gist/blob/main/CHANGELOG.md",
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} git-gist contributors.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ["bash", "toml", "json"],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
