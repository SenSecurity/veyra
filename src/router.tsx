import {
  createRouter,
  createRootRoute,
  createRoute,
} from "@tanstack/react-router";
import { App } from "./app";
import { HomeRoute } from "@/routes";
import { HistoryRoute } from "@/routes/history";
import { EmailDraftsRoute } from "@/routes/email-drafts";
import { DictionaryRoute } from "@/routes/dictionary";
import { WizardRoute } from "@/routes/wizard";
import { SettingsLayout } from "@/routes/settings/layout";
import { SettingsGeneralRoute } from "@/routes/settings/general";
import { SettingsTranscriptionRoute } from "@/routes/settings/transcription";
import { SettingsHotkeysRoute } from "@/routes/settings/hotkeys";
import { SettingsOverlayRoute } from "@/routes/settings/overlay";

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

const emailDraftsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/email-drafts",
  component: EmailDraftsRoute,
});

const dictionaryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/dictionary",
  component: DictionaryRoute,
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
  component: SettingsOverlayRoute,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  historyRoute,
  emailDraftsRoute,
  dictionaryRoute,
  wizardRoute,
  settingsRoute.addChildren([
    settingsGeneralRoute,
    settingsTranscriptionRoute,
    settingsHotkeysRoute,
    settingsOverlayRoute,
  ]),
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
