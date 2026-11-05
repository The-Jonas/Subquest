<!-- FontPicker — one list with the built-in, Windows and imported fonts, grouped. -->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { builtInFonts, systemFonts, importedOption, type FontOption } from "../fonts";
  import type { ImportedFont } from "../api";

  let {
    value = $bindable(""),
    imported = [],
    onchange,
  }: { value?: string; imported?: ImportedFont[]; onchange?: (value: string) => void } = $props();

  const importedOptions = $derived<FontOption[]>(imported.map(importedOption));
</script>

<select class="input" bind:value onchange={() => onchange?.(value)}>
  <optgroup label={t("fonts.groups.builtIn")}>
    {#each builtInFonts as f (f.value)}
      <option value={f.value}>{f.value === "" ? t("fonts.default", { name: f.label }) : f.label}</option>
    {/each}
  </optgroup>
  <optgroup label={t("fonts.groups.windows")}>
    {#each systemFonts as f (f.value)}<option value={f.value}>{f.label}</option>{/each}
  </optgroup>
  {#if importedOptions.length}
    <optgroup label={t("fonts.groups.imported")}>
      {#each importedOptions as f (f.value)}<option value={f.value}>{f.label}</option>{/each}
    </optgroup>
  {/if}
</select>
