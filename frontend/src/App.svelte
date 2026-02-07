<script lang="ts">
  import { getRoute } from "./lib/router.svelte";
  import { connect, disconnect } from "./lib/ws";
  import { healthCheck } from "./lib/api";
  import ReviewList from "./components/ReviewList.svelte";
  import ReviewView from "./components/ReviewView.svelte";
  import ConnectionStatus from "./components/ConnectionStatus.svelte";

  const route = $derived(getRoute());

  let version = $state("");

  $effect(() => {
    connect();
    healthCheck()
      .then((res) => (version = res.version))
      .catch(() => {});
    return () => disconnect();
  });
</script>

{#if route.page === "review" && route.reviewId}
  <ReviewView reviewId={route.reviewId} />
{:else}
  <ReviewList />
{/if}

<ConnectionStatus {version} />
