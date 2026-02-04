<script lang="ts">
  import { getRoute } from "./lib/router.svelte";
  import ReviewList from "./components/ReviewList.svelte";
  import ReviewView from "./components/ReviewView.svelte";

  const route = $derived(getRoute());
</script>

{#if route.page === "review" && route.reviewId}
  <ReviewView reviewId={route.reviewId} />
{:else}
  <ReviewList />
{/if}
