<script lang="ts">
  let status = $state("checking...");

  async function checkHealth() {
    try {
      const res = await fetch("/api/health");
      status = res.ok ? "connected" : `error: ${res.status}`;
    } catch {
      status = "disconnected";
    }
  }

  $effect(() => {
    checkHealth();
  });
</script>

<main>
  <h1>hello preflight</h1>
  <p>backend: {status}</p>
</main>
