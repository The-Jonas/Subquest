// tour.svelte.ts — the guided tour: it opens each screen for real and points
// at the part being explained (an element marked with data-tour="<target>").
// Runs by itself the first time, and again from the sidebar's tour button.
import { app, persistSettings, type Screen, type SettingsTab } from "./app.svelte";

export interface TourStep {
  key: string; // texts in tour.steps.<key>.title / .text
  screen: Screen; // screen opened for this step
  tab?: SettingsTab; // Settings part, when the screen is Settings
  target?: string; // data-tour value to point at (none: bubble in the middle)
}

export const tourSteps: TourStep[] = [
  { key: "welcome", screen: "play" },
  { key: "translator", screen: "settings", tab: "translation", target: "translator" },
  { key: "quick", screen: "settings", tab: "translation", target: "quick" },
  { key: "game", screen: "play", target: "play-box" },
  { key: "styles", screen: "games", target: "games-take" },
  { key: "glossary", screen: "games", target: "glossary" },
  { key: "toolbar", screen: "settings", tab: "shortcuts", target: "shortcuts" },
  { key: "history", screen: "history", target: "nav-history" },
  { key: "done", screen: "play", target: "nav-tour" },
];

export const tour = $state({
  active: false,
  index: 0,
});

// Opens the screen (and Settings part) of the current step.
function show() {
  const step = tourSteps[tour.index];
  app.screen = step.screen;
  if (step.tab) app.settingsTab = step.tab;
}

// Starts the tour from the first step.
export function startTour() {
  tour.index = 0;
  tour.active = true;
  show();
}

// Moves one step forward or back (within the tour).
export function moveTour(by: 1 | -1) {
  tour.index = Math.min(tourSteps.length - 1, Math.max(0, tour.index + by));
  show();
}

// Ends the tour (finished or skipped): it doesn't start by itself anymore.
export async function endTour() {
  tour.active = false;
  app.screen = "play";
  if (!app.settings.tutorialDone) {
    app.settings.tutorialDone = true;
    await persistSettings().catch(() => {});
  }
}
