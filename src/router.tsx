import {
  createRouter,
  createRootRoute,
  createRoute,
} from "@tanstack/react-router";
import { App } from "./app";
import { HomeRoute } from "@/routes";
import { HistoryRoute } from "@/routes/history";
import { DictionaryRoute } from "@/routes/dictionary";
import { SnippetsRoute } from "@/routes/snippets";
import { ScratchpadRoute } from "@/routes/scratchpad";
import { WizardRoute } from "@/routes/wizard";
import { SettingsLayout } from "@/routes/settings/layout";
import { SettingsGeneralRoute } from "@/routes/settings/general";
import { SettingsTranscriptionRoute } from "@/routes/settings/transcription";
import { SettingsHotkeysRoute } from "@/routes/settings/hotkeys";
import { SettingsAboutRoute, SettingsSimpleRoute } from "@/routes/settings/simple";

// Root + chrome
const rootRoute = createRootRoute({ component: App });

// Top-level routes
const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: HomeRoute,
});

const historyRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/history",
  component: HistoryRoute,
});

const dictionaryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/dictionary",
  component: DictionaryRoute,
});

const snippetsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/snippets",
  component: SnippetsRoute,
});

const scratchpadRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/scratchpad",
  component: ScratchpadRoute,
});

const wizardRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/wizard",
  component: WizardRoute,
});

// Settings parent + nested tabs
const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsLayout,
});

const settingsGeneralRoute = createRoute({
  getParentRoute: () => settingsRoute,
  path: "/general",
  component: SettingsGeneralRoute,
});

const settingsTranscriptionRoute = createRoute({
  getParentRoute: () => settingsRoute,
  path: "/transcription",
  component: SettingsTranscriptionRoute,
});

const settingsHotkeysRoute = createRoute({
  getParentRoute: () => settingsRoute,
  path: "/hotkeys",
  component: SettingsHotkeysRoute,
});

const settingsOverlayRoute = createRoute({
  getParentRoute: () => settingsRoute,
  path: "/overlay",
  component: () => <SettingsSimpleRoute title="Overlay" text="Overlay visual and monitor options." />,
});

const settingsFormattingRoute = createRoute({
  getParentRoute: () => settingsRoute,
  path: "/formatting",
  component: () => <SettingsSimpleRoute title="Formatting" text="Filler removal, punctuation, and text cleanup." />,
});

const settingsSystemRoute = createRoute({
  getParentRoute: () => settingsRoute,
  path: "/system",
  component: () => <SettingsSimpleRoute title="System" text="Startup, logs, and local runtime controls." />,
});

const settingsStatsRoute = createRoute({
  getParentRoute: () => settingsRoute,
  path: "/stats",
  component: () => <SettingsSimpleRoute title="Stats" text="Milestones and analytics preferences." />,
});

const settingsDataRoute = createRoute({
  getParentRoute: () => settingsRoute,
  path: "/data",
  component: () => <SettingsSimpleRoute title="Data" text="Retention and export controls." />,
});

const settingsAboutRoute = createRoute({
  getParentRoute: () => settingsRoute,
  path: "/about",
  component: SettingsAboutRoute,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  historyRoute,
  dictionaryRoute,
  snippetsRoute,
  scratchpadRoute,
  wizardRoute,
  settingsRoute.addChildren([
    settingsGeneralRoute,
    settingsTranscriptionRoute,
    settingsHotkeysRoute,
    settingsOverlayRoute,
    settingsFormattingRoute,
    settingsSystemRoute,
    settingsStatsRoute,
    settingsDataRoute,
    settingsAboutRoute,
  ]),
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
