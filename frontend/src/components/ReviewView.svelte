<script lang="ts">
  interface Props {
    reviewId: string;
  }

  let { reviewId }: Props = $props();
</script>

<div class="flex items-center justify-center min-h-screen">
  <p class="text-text-muted">Review {reviewId} — coming soon</p>
</div>
