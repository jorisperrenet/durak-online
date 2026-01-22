<script>
  import Card from './Card.svelte'
  import { createEventDispatcher } from 'svelte'
  
  export let availableRanks = []
  export let suits = ['Clubs', 'Diamonds', 'Hearts', 'Spades']
  export let selectedCard = null
  
  const dispatch = createEventDispatcher()
  
  function selectCard(suit, rank) {
    const card = { suit, rank }
    selectedCard = card
    dispatch('select', card)
  }
  
  function cardKey(suit, rank) {
    return `${suit}:${rank}`
  }
  
  function isSelected(suit, rank) {
    return selectedCard && selectedCard.suit === suit && selectedCard.rank === rank
  }
</script>

<div class="space-y-3">
  {#each suits as suit}
    <div>
      <div class="text-xs text-zinc-400 mb-2">{suit}</div>
      <div class="flex flex-wrap gap-2">
        {#each availableRanks as rank}
          {@const card = { suit, rank }}
          {@const selected = isSelected(suit, rank)}
          <button
            class={`transition-all ${selected ? 'ring-2 ring-indigo-500 scale-105' : ''}`}
            on:click={() => selectCard(suit, rank)}
            type="button"
          >
            <Card card={card} size="sm" selectable={true} selected={selected} />
          </button>
        {/each}
      </div>
    </div>
  {/each}
</div>
