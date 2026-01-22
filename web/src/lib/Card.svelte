<script>
  import { cardImageUrl, cardBackUrl, cardLabel } from './cards.js'

  export let card = null
  export let faceDown = false
  export let selectable = false
  export let selected = false
  export let disabled = false
  export let size = 'md' // 'sm'|'md'|'lg'
  export let known = false // true = card is known but physically hidden (show with overlay)
  export let revealed = false // true = your card that is known to opponents

  const sizes = {
    sm: 'w-12 h-16',
    md: 'w-16 h-24',
    lg: 'w-24 h-36',
  }
</script>

<button
  class={`relative ${sizes[size] || sizes.md} overflow-hidden rounded-md border border-zinc-700/70 bg-zinc-50 p-0.5 shadow-md shadow-black/40 ${
    selectable && !disabled ? 'hover:-translate-y-1 hover:border-zinc-500' : ''
  } ${selected ? 'ring-2 ring-indigo-500' : ''} ${disabled ? 'opacity-60' : ''} ${known ? 'ring-1 ring-amber-500/50' : ''} ${revealed ? 'ring-1 ring-amber-500/60' : ''}`}
  disabled={disabled}
  on:click
  aria-label={faceDown ? 'Face-down card' : cardLabel(card)}
>
  <img
    class="h-full w-full rounded-[4px] bg-zinc-50 object-contain"
    src={faceDown ? cardBackUrl() : cardImageUrl(card)}
    alt={faceDown ? 'Card back' : cardLabel(card)}
    draggable="false"
    loading="lazy"
  />
  {#if known}
    <!-- Semi-transparent overlay to indicate this card is known but hidden -->
    <div class="absolute inset-0 bg-black/30 rounded-[4px] pointer-events-none"></div>
    <div class="absolute bottom-0 left-0 right-0 bg-amber-500/80 text-[8px] text-center text-black font-medium py-0.5">
      known
    </div>
  {/if}
  {#if revealed}
    <!-- Indicator that this card is known to opponents -->
    <div class="absolute top-0 right-0 bg-amber-500/80 text-[7px] text-black font-medium px-1 py-0.5 rounded-bl">
      !
    </div>
  {/if}
</button>
