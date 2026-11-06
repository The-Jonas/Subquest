<!--
  FontDetect — finds the game's font. Takes a full-size crop of the game text
  (`snapshot`), asks the translator model for the closest Google Fonts and
  shows each one writing the line the model read, so the user compares it with
  the game by eye. "Use this one" keeps the file and picks it for the area,
  along with the letter case the model saw (the user can change it).
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { pictureUrl, detectFont, keepCandidateFont, errorKind, type FontDetection, type LetterCase } from "../api";
  import { candidateFamily, importedOption, loadCandidateFont, loadImportedFont } from "../fonts";

  let {
    snapshot,
    value = $bindable(""),
    caps = $bindable<LetterCase>(""),
    title = "",
    help = "",
    showCrop = true,
    onchoose,
  }: {
    snapshot: () => Promise<Blob>;
    value?: string;
    caps?: LetterCase;
    title?: string; // replaces the default heading
    help?: string; // replaces the default explanation
    showCrop?: boolean; // false when the picture is already shown next to it
    onchoose?: (value: string, caps: LetterCase) => void; // a suggestion was kept
  } = $props();

  const cases: LetterCase[] = ["", "upper", "smallCaps"];

  let busy = $state(false);
  let error = $state<string | null>(null);
  let result = $state<FontDetection | null>(null);
  let crop = $state<string | null>(null); // object URL of the image that was sent
  let keeping = $state<string | null>(null); // file being saved

  // Line drawn by every candidate: what the model read, or a stock sample.
  const sample = $derived(result?.text || t("fonts.sampleText"));

  // Crops the game, asks for suggestions and loads each one so its sample renders.
  async function detect() {
    busy = true;
    error = null;
    result = null;
    let png: Blob;
    try {
      png = await snapshot();
    } catch {
      error = t("fonts.detect.snapshotFailed");
      busy = false;
      return;
    }
    crop = await pictureUrl(png).catch(() => null);
    try {
      const found = await detectFont(png);
      await Promise.all(found.candidates.map((c) => loadCandidateFont(c.file).catch(() => {})));
      result = found;
      caps = found.caps; // the samples show the game's case right away
    } catch (e) {
      error = t(`errors.${errorKind(e)}`);
    } finally {
      busy = false;
    }
  }

  // Saves the candidate in the Fonts folder and selects it for this area.
  async function use(file: string) {
    keeping = file;
    error = null;
    try {
      const font = await keepCandidateFont(file);
      await loadImportedFont(font.file).catch(() => {});
      value = importedOption(font).value;
      onchoose?.(value, caps);
    } catch (e) {
      error = t(`errors.${errorKind(e)}`);
    } finally {
      keeping = null;
    }
  }

</script>

<section class="detect">
  <header>
    <div>
      <h3>{title || t("fonts.detect.title")}</h3>
      <p class="help">{help || t("fonts.detect.help")}</p>
    </div>
    <button class="btn btn-ghost" onclick={detect} disabled={busy}>
      {result ? t("fonts.detect.again") : t("fonts.detect.button")}
    </button>
  </header>

  {#if busy}
    <p class="working" aria-live="polite">{t("fonts.detect.working")}</p>
  {/if}

  {#if result}
    <!-- The game's own pixels first, then each candidate writing the same line -->
    <div class="reference">
      {#if crop && showCrop}
        <img class="crop" src={crop} alt="" />
      {:else}
        <span></span>
      {/if}
      <label class="case">
        <span>{t("fonts.detect.case")}</span>
        <select class="input" bind:value={caps}>
          {#each cases as c (c)}<option value={c}>{t(`fonts.detect.cases.${c || "normal"}`)}</option>{/each}
        </select>
      </label>
    </div>
    <ul class="candidates">
      {#each result.candidates as c (c.file)}
        {@const chosen = value === `imported:${c.file}`}
        <li class:chosen>
          <p
            class="sample"
            class:upper={caps === "upper"}
            class:small-caps={caps === "smallCaps"}
            style:font-family={candidateFamily(c.file, c.family)}
            style:font-weight={c.weight}
          >
            {sample}
          </p>
          <div class="meta">
            <div>
              <strong>{c.family}</strong>
              <span>{t("fonts.detect.weight", { weight: String(c.weight) })}</span>
              {#if c.reason}<span class="reason">{c.reason}</span>{/if}
            </div>
            {#if chosen}
              <span class="in-use">{t("fonts.detect.inUse")}</span>
            {:else}
              <button class="btn" onclick={() => use(c.file)} disabled={keeping !== null}>
                {t("fonts.detect.use")}
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</section>

<style>
  .detect {
    display: grid;
    gap: 12px;
    margin-top: 14px;
    padding: 14px 16px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--night-deep);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }
  h3 {
    margin: 0;
    font-size: var(--text-md, 1rem);
  }
  .help {
    margin: 2px 0 0;
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  header .btn {
    flex-shrink: 0;
  }

  /* Waiting line with the dialogue-box caret, so it reads as the app "thinking" */
  .working {
    margin: 0;
    color: var(--live);
    font-size: var(--text-sm);
  }
  .working::after {
    content: " ▼";
    animation: blink 1s steps(2) infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }

  /* The real game text, as the reference to compare against */
  .crop {
    display: block;
    max-width: 100%;
    max-height: 120px;
    object-fit: contain;
    object-position: left center;
    border-radius: 6px;
    border: 1px solid var(--line);
  }

  .reference {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
  }
  .case {
    display: grid;
    gap: 4px;
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  .upper {
    text-transform: uppercase;
  }
  .small-caps {
    font-variant-caps: small-caps;
  }

  .candidates {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  .candidates li {
    display: grid;
    gap: 8px;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
  }
  .candidates li.chosen {
    border-color: var(--gold);
    box-shadow: inset 0 0 0 1px var(--gold);
  }
  .sample {
    margin: 0;
    font-size: 26px;
    line-height: 1.2;
    overflow-wrap: anywhere;
  }
  .meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  .meta div {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: baseline;
    font-size: var(--text-sm);
    color: var(--ink-dim);
  }
  .meta strong {
    color: var(--ink);
  }
  .reason {
    color: var(--ink-faint);
  }
  .in-use {
    color: var(--gold);
    font-size: var(--text-sm);
    font-weight: 700;
  }
  .error {
    margin: 0;
    color: var(--danger);
    font-size: var(--text-sm);
  }
</style>
