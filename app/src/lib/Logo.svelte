<script lang="ts">
  // Same drawing as assets/icon.svg.
  let { size = 24 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
  <path d="M15 5h25l13 13v36a5 5 0 0 1-5 5H15a5 5 0 0 1-5-5V10a5 5 0 0 1 5-5z" fill="#fffdfa" stroke="#2b2622" stroke-width="3" stroke-linejoin="round" />
  <path d="M40 5v8a5 5 0 0 0 5 5h8" fill="#f6dccd" stroke="#2b2622" stroke-width="3" stroke-linejoin="round" />
  <circle cx="27.5" cy="40" r="8" fill="none" stroke="var(--accent, #e07a4f)" stroke-width="5" />
  <rect x="33" y="21" width="5" height="27.5" rx="2.5" fill="var(--accent, #e07a4f)" />
</svg>
