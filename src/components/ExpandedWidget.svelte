<script lang="ts">
  import Header from "./expanded/Header.svelte";
  import Preview from "./expanded/Preview.svelte";
  import SidePanel from "./expanded/SidePanel.svelte";
  import Transport from "./expanded/Transport.svelte";
  import Timeline from "./expanded/Timeline.svelte";
</script>

<div class="expanded">
  <Header />
  <div class="main">
    <Preview />
    <SidePanel />
  </div>
  <Transport />
  <Timeline />
</div>

<style>
  .expanded {
    width: 940px;
    display: flex;
    flex-direction: column;
  }
  .main {
    display: grid;
    grid-template-columns: 600px 1fr;
    border-bottom: 2px solid var(--color-divider);
  }
</style>
