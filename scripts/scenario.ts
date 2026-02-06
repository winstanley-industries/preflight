import { execSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { createInterface } from "node:readline/promises";

const BASE_URL = process.env.PREFLIGHT_URL || "http://127.0.0.1:3000";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const rl = createInterface({ input: process.stdin, output: process.stdout });

function log(phase: string, msg: string): void {
  console.log(`\n  \x1b[36m[${phase}]\x1b[0m ${msg}`);
}

function banner(text: string): void {
  console.log(`\n\x1b[1m${"=".repeat(60)}\x1b[0m`);
  console.log(`\x1b[1m  ${text}\x1b[0m`);
  console.log(`\x1b[1m${"=".repeat(60)}\x1b[0m`);
}

async function pause(msg: string): Promise<void> {
  console.log(`\n  \x1b[33m${msg}\x1b[0m`);
  await rl.question("  Press Enter to continue...");
}

function git(repoPath: string, ...args: string[]): string {
  return execSync(`git ${args.join(" ")}`, {
    cwd: repoPath,
    encoding: "utf-8",
    stdio: ["pipe", "pipe", "pipe"],
  }).trim();
}

async function api<T = unknown>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const opts: RequestInit = {
    method,
    headers: { "Content-Type": "application/json" },
  };
  if (body !== undefined) opts.body = JSON.stringify(body);
  const res = await fetch(`${BASE_URL}${path}`, opts);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${method} ${path} → ${res.status}: ${text}`);
  }
  if (res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
}

function writeFiles(repoPath: string, files: Record<string, string>): void {
  for (const [name, content] of Object.entries(files)) {
    writeFileSync(join(repoPath, name), content);
  }
}

// ---------------------------------------------------------------------------
// File contents — a silly "Space Cats" landing page
// ---------------------------------------------------------------------------

// Base commit (the "before" state)
const BASE_FILES: Record<string, string> = {
  "index.html": `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Space Cats</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <h1>Welcome to Space Cats</h1>
  <p>The internet's premier space cat directory.</p>
  <script src="app.js"></script>
</body>
</html>
`,
  "style.css": `body {
  font-family: Arial, sans-serif;
  margin: 0;
  padding: 20px;
  background: #111;
  color: #eee;
}

h1 {
  color: #ff6600;
}
`,
  "app.js": `"use strict";

console.log("Space Cats loaded");
`,
};

// Revision 1 — add navigation, hero section, cat facts
const REV1_FILES: Record<string, string> = {
  "index.html": `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Space Cats</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <div class="nav">
    <a href="#" class="logo">Space Cats</a>
    <a href="#facts">Facts</a>
    <a href="#gallery">Gallery</a>
  </div>

  <div class="hero">
    <h1>Cats. In. Space.</h1>
    <p>Exploring the final frontier, one paw at a time.</p>
  </div>

  <section id="facts">
    <h2>Cat Facts</h2>
    <p id="fact-display">Loading cat fact...</p>
  </section>
  <script src="app.js"></script>
</body>
</html>
`,
  "style.css": `body {
  font-family: Arial, sans-serif;
  margin: 0;
  padding: 0;
  background: #111;
  color: #eee;
}

.nav {
  display: flex;
  gap: 20px;
  padding: 16px 24px;
  background: #1a1a2e;
  border-bottom: 2px solid #ff6600;
}

.nav a {
  color: #ccc;
  text-decoration: none;
}

.nav .logo {
  font-weight: bold;
  color: #ff6600;
  margin-right: auto;
}

.hero {
  text-align: center;
  padding: 80px 20px;
  background: linear-gradient(135deg, #0f0c29, #302b63, #24243e);
}

.hero h1 {
  font-size: 3rem;
  color: #ff6600;
  margin-bottom: 10px;
}

#facts {
  padding: 40px 24px;
}
`,
  "app.js": `"use strict";

const catFacts = [
  "Cats can rotate their ears 180 degrees.",
  "A group of cats is called a clowder.",
  "Cats spend 70% of their lives sleeping.",
  "The first cat in space was French. Her name was Felicette.",
  "Cats can jump up to 6 times their length.",
];

function rotateFacts() {
  const el = document.getElementById("fact-display");
  let i = 0;
  setInterval(() => {
    el.textContent = catFacts[i];
    i = (i + 1) % catFacts.length;
  }, 3000);
}

rotateFacts();
`,
};

// Revision 2 — add footer, meet-the-cats section, dark mode toggle
const REV2_FILES: Record<string, string> = {
  "index.html": `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Space Cats</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <nav class="nav">
    <a href="#" class="logo">Space Cats</a>
    <a href="#facts">Facts</a>
    <a href="#cats">Cats</a>
    <button id="theme-toggle" aria-label="Toggle theme">Light Mode</button>
  </nav>

  <div class="hero">
    <h1>Cats. In. Space.</h1>
    <p>Exploring the final frontier, one paw at a time.</p>
  </div>

  <section id="facts">
    <h2>Cat Facts</h2>
    <p id="fact-display">Loading cat fact...</p>
  </section>

  <section id="cats">
    <h2>Meet the Crew</h2>
    <div class="card-grid">
      <div class="card">
        <h3>Commander Whiskers</h3>
        <p>Chief Navigation Officer. Expert in zero-gravity napping.</p>
      </div>
      <div class="card">
        <h3>Lieutenant Mittens</h3>
        <p>Chief Science Officer. Has knocked 3 beakers off the ISS counter.</p>
      </div>
      <div class="card">
        <h3>Ensign Fluffernaut</h3>
        <p>Rookie astronaut. Still figuring out the litter box in microgravity.</p>
      </div>
    </div>
  </section>

  <footer class="footer">
    <p>Space Cats &copy; 2026. No cats were harmed in the making of this website.</p>
  </footer>
  <script src="app.js"></script>
</body>
</html>
`,
  "style.css": `body {
  font-family: Arial, sans-serif;
  margin: 0;
  padding: 0;
  background: var(--bg, #111);
  color: var(--text, #eee);
  transition: background 0.3s, color 0.3s;
}

body.light {
  --bg: #f5f5f5;
  --text: #222;
}

.nav {
  display: flex;
  gap: 20px;
  align-items: center;
  padding: 16px 24px;
  background: #1a1a2e;
  border-bottom: 2px solid #ff6600;
}

.nav a {
  color: #ccc;
  text-decoration: none;
}

.nav .logo {
  font-weight: bold;
  color: #ff6600;
  margin-right: auto;
}

#theme-toggle {
  background: #ff6600;
  color: white;
  border: none;
  padding: 6px 14px;
  border-radius: 4px;
  cursor: pointer;
}

.hero {
  text-align: center;
  padding: 80px 20px;
  background: linear-gradient(135deg, #0f0c29, #302b63, #24243e);
}

.hero h1 {
  font-size: 3rem;
  color: #ff6600;
  margin-bottom: 10px;
}

#facts, #cats {
  padding: 40px 24px;
}

.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 20px;
  margin-top: 20px;
}

.card {
  background: #1a1a2e;
  border: 1px solid #333;
  border-radius: 8px;
  padding: 20px;
}

.card h3 {
  color: #ff6600;
  margin-top: 0;
}

.footer {
  text-align: center;
  padding: 24px;
  border-top: 1px solid #333;
  color: #888;
  font-size: 0.9rem;
}
`,
  "app.js": `"use strict";

const catFacts = [
  "Cats can rotate their ears 180 degrees.",
  "A group of cats is called a clowder.",
  "Cats spend 70% of their lives sleeping.",
  "The first cat in space was French. Her name was Felicette.",
  "Cats can jump up to 6 times their length.",
];

function rotateFacts() {
  const el = document.getElementById("fact-display");
  if (!el) return;
  let i = 0;
  const timer = setInterval(() => {
    el.textContent = catFacts[i];
    i = (i + 1) % catFacts.length;
  }, 3000);
  return () => clearInterval(timer);
}

function initThemeToggle() {
  const btn = document.getElementById("theme-toggle");
  if (!btn) return;
  btn.addEventListener("click", () => {
    document.body.classList.toggle("light");
    btn.textContent = document.body.classList.contains("light")
      ? "Dark Mode"
      : "Light Mode";
  });
}

rotateFacts();
initThemeToggle();
`,
};

// Revision 3 — add contact form with validation
const REV3_FILES: Record<string, string> = {
  "index.html": `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Space Cats</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <nav class="nav">
    <a href="#" class="logo">Space Cats</a>
    <a href="#facts">Facts</a>
    <a href="#cats">Cats</a>
    <a href="#contact">Contact</a>
    <button id="theme-toggle" aria-label="Toggle theme">Light Mode</button>
  </nav>

  <div class="hero">
    <h1>Cats. In. Space.</h1>
    <p>Exploring the final frontier, one paw at a time.</p>
  </div>

  <section id="facts">
    <h2>Cat Facts</h2>
    <p id="fact-display">Loading cat fact...</p>
  </section>

  <section id="cats">
    <h2>Meet the Crew</h2>
    <div class="card-grid">
      <div class="card">
        <h3>Commander Whiskers</h3>
        <p>Chief Navigation Officer. Expert in zero-gravity napping.</p>
      </div>
      <div class="card">
        <h3>Lieutenant Mittens</h3>
        <p>Chief Science Officer. Has knocked 3 beakers off the ISS counter.</p>
      </div>
      <div class="card">
        <h3>Ensign Fluffernaut</h3>
        <p>Rookie astronaut. Still figuring out the litter box in microgravity.</p>
      </div>
    </div>
  </section>

  <section id="contact">
    <h2>Report a Space Cat Sighting</h2>
    <form id="sighting-form">
      <label for="name">Your Name</label>
      <input type="text" id="name" name="name" required>
      <label for="location">Sighting Location</label>
      <input type="text" id="location" name="location" required>
      <label for="description">Description</label>
      <textarea id="description" name="description" rows="4" required></textarea>
      <button type="submit">Submit Sighting</button>
      <p id="form-error" class="error" hidden></p>
      <p id="form-success" class="success" hidden>Sighting reported. Thank you, citizen.</p>
    </form>
  </section>

  <footer class="footer">
    <p>Space Cats &copy; 2026. No cats were harmed in the making of this website.</p>
  </footer>
  <script src="app.js"></script>
</body>
</html>
`,
  "style.css": `body {
  font-family: Arial, sans-serif;
  margin: 0;
  padding: 0;
  background: var(--bg, #111);
  color: var(--text, #eee);
  transition: background 0.3s, color 0.3s;
}

body.light {
  --bg: #f5f5f5;
  --text: #222;
}

.nav {
  display: flex;
  gap: 20px;
  align-items: center;
  padding: 16px 24px;
  background: #1a1a2e;
  border-bottom: 2px solid #ff6600;
}

.nav a {
  color: #ccc;
  text-decoration: none;
}

.nav .logo {
  font-weight: bold;
  color: #ff6600;
  margin-right: auto;
}

#theme-toggle {
  background: #ff6600;
  color: white;
  border: none;
  padding: 6px 14px;
  border-radius: 4px;
  cursor: pointer;
}

.hero {
  text-align: center;
  padding: 80px 20px;
  background: linear-gradient(135deg, #0f0c29, #302b63, #24243e);
}

.hero h1 {
  font-size: 3rem;
  color: #ff6600;
  margin-bottom: 10px;
}

#facts, #cats, #contact {
  padding: 40px 24px;
}

.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 20px;
  margin-top: 20px;
}

.card {
  background: #1a1a2e;
  border: 1px solid #333;
  border-radius: 8px;
  padding: 20px;
}

.card h3 {
  color: #ff6600;
  margin-top: 0;
}

form {
  max-width: 500px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

form label {
  font-weight: bold;
  font-size: 0.9rem;
}

form input, form textarea {
  padding: 10px;
  border: 1px solid #444;
  border-radius: 4px;
  background: #1a1a2e;
  color: #eee;
  font-size: 1rem;
}

form button {
  background: #ff6600;
  color: white;
  border: none;
  padding: 10px 20px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 1rem;
}

.error { color: #ff4444; }
.success { color: #44ff44; }

.footer {
  text-align: center;
  padding: 24px;
  border-top: 1px solid #333;
  color: #888;
  font-size: 0.9rem;
}
`,
  "app.js": `"use strict";

const catFacts = [
  "Cats can rotate their ears 180 degrees.",
  "A group of cats is called a clowder.",
  "Cats spend 70% of their lives sleeping.",
  "The first cat in space was French. Her name was Felicette.",
  "Cats can jump up to 6 times their length.",
];

function rotateFacts() {
  const el = document.getElementById("fact-display");
  if (!el) return;
  let i = 0;
  const timer = setInterval(() => {
    el.textContent = catFacts[i];
    i = (i + 1) % catFacts.length;
  }, 3000);
  return () => clearInterval(timer);
}

function initThemeToggle() {
  const btn = document.getElementById("theme-toggle");
  if (!btn) return;
  btn.addEventListener("click", () => {
    document.body.classList.toggle("light");
    btn.textContent = document.body.classList.contains("light")
      ? "Dark Mode"
      : "Light Mode";
  });
}

function initContactForm() {
  const form = document.getElementById("sighting-form");
  if (!form) return;

  form.addEventListener("submit", (e) => {
    e.preventDefault();
    const name = form.querySelector("#name").value.trim();
    const location = form.querySelector("#location").value.trim();
    const description = form.querySelector("#description").value.trim();
    const errorEl = form.querySelector("#form-error");
    const successEl = form.querySelector("#form-success");

    errorEl.hidden = true;
    successEl.hidden = true;

    if (description.length < 10) {
      errorEl.textContent = "Please describe the sighting in at least 10 characters.";
      errorEl.hidden = false;
      return;
    }

    // Simulate submission
    successEl.hidden = false;
    form.reset();
  });
}

rotateFacts();
initThemeToggle();
initContactForm();
`,
};

// ---------------------------------------------------------------------------
// Types for API responses
// ---------------------------------------------------------------------------

interface ReviewResponse {
  id: string;
  title: string | null;
  status: string;
  file_count: number;
  thread_count: number;
  revision_count: number;
}

interface ThreadResponse {
  id: string;
  review_id: string;
  file_path: string;
  line_start: number;
  line_end: number;
  status: string;
}

// ---------------------------------------------------------------------------
// Scenario phases
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  banner("Space Cats Scenario");
  console.log(`  Server: ${BASE_URL}`);

  // Verify server is reachable
  try {
    await fetch(`${BASE_URL}/api/reviews`);
  } catch {
    console.error(`\n  \x1b[31mCannot reach server at ${BASE_URL}.\x1b[0m`);
    console.error("  Start it first with: just run");
    process.exit(1);
  }

  // -----------------------------------------------------------------------
  // Setup: create temp git repo
  // -----------------------------------------------------------------------
  const repoPath = mkdtempSync(join(tmpdir(), "preflight-scenario-"));
  log("setup", `Temp repo: ${repoPath}`);

  git(repoPath, "init");
  git(repoPath, "config", "user.email", "spacecats@example.com");
  git(repoPath, "config", "user.name", "Space Cats CI");

  writeFiles(repoPath, BASE_FILES);
  git(repoPath, "add", ".");
  git(repoPath, "commit", "-m", "Initial commit: bare-bones Space Cats site");
  log("setup", "Base commit created");

  // -----------------------------------------------------------------------
  // Phase 1: Create review with first revision
  // -----------------------------------------------------------------------
  banner("Phase 1 — Initial Review");
  log("phase-1", "Modifying files for revision 1 (nav, hero, cat facts)...");
  writeFiles(repoPath, REV1_FILES);

  log("phase-1", "Creating review...");
  const review = await api<ReviewResponse>("POST", "/api/reviews", {
    title: "Add landing page for Space Cats",
    repo_path: repoPath,
    base_ref: "HEAD",
  });
  log("phase-1", `Review created: ${review.id}`);
  log("phase-1", `  ${review.file_count} files, revision 1`);

  await pause("Check the UI — you should see the review in the list. Click into it to see the diff.");

  // -----------------------------------------------------------------------
  // Phase 2: Code review feedback (threads + comments)
  // -----------------------------------------------------------------------
  banner("Phase 2 — Code Review Feedback");

  log("phase-2", "Adding thread: semantic HTML suggestion on index.html...");
  const thread1 = await api<ThreadResponse>(
    "POST",
    `/api/reviews/${review.id}/threads`,
    {
      file_path: "index.html",
      line_start: 10,
      line_end: 13,
      origin: "Comment",
      body: "Use semantic <nav> instead of <div class=\"nav\">. Better for accessibility and SEO.",
      author_type: "Human",
    },
  );

  log("phase-2", "Adding thread: setInterval cleanup warning on app.js...");
  const thread2 = await api<ThreadResponse>(
    "POST",
    `/api/reviews/${review.id}/threads`,
    {
      file_path: "app.js",
      line_start: 13,
      line_end: 18,
      origin: "Comment",
      body: "This setInterval is never cleaned up. If this component ever unmounts, you'll leak timers. Return the cleanup function or store the timer ID.",
      author_type: "Human",
    },
  );

  log("phase-2", "Adding thread: CSS suggestion on style.css...");
  const thread3 = await api<ThreadResponse>(
    "POST",
    `/api/reviews/${review.id}/threads`,
    {
      file_path: "style.css",
      line_start: 6,
      line_end: 8,
      origin: "Comment",
      body: "Consider using CSS custom properties for the color palette. The orange (#ff6600) is repeated in several places — a single --accent variable would make theming easier.",
      author_type: "Agent",
    },
  );

  log("phase-2", `3 threads created: ${thread1.id}, ${thread2.id}, ${thread3.id}`);

  // Add a reply to thread 1
  log("phase-2", "Agent replies to the semantic HTML thread...");
  await api("POST", `/api/threads/${thread1.id}/comments`, {
    author_type: "Agent",
    body: "Good catch! I'll switch to <nav> in the next revision.",
  });

  await pause("Check the UI — click into files to see the review threads and comments.");

  // -----------------------------------------------------------------------
  // Phase 3: Address feedback — Revision 2
  // -----------------------------------------------------------------------
  banner("Phase 3 — Revision 2: Address Feedback");

  log("phase-3", "Modifying files for revision 2 (semantic nav, footer, dark mode, cleanup)...");
  writeFiles(repoPath, REV2_FILES);

  log("phase-3", "Creating revision 2...");
  const rev2 = await api<{ revision_number: number; file_count: number }>(
    "POST",
    `/api/reviews/${review.id}/revisions`,
    {
      trigger: "Agent",
      message: "Address review feedback: semantic HTML, timer cleanup, dark mode toggle",
    },
  );
  log("phase-3", `Revision ${rev2.revision_number} created (${rev2.file_count} files)`);

  await pause("Check the UI — switch between revisions to see the diff change.");

  // -----------------------------------------------------------------------
  // Phase 4: More discussion + resolve threads
  // -----------------------------------------------------------------------
  banner("Phase 4 — Discussion & Resolve Threads");

  log("phase-4", "Human confirms semantic HTML fix...");
  await api("POST", `/api/threads/${thread1.id}/comments`, {
    author_type: "Human",
    body: "Looks good, <nav> is much better. Resolving.",
  });
  log("phase-4", "Resolving thread 1 (semantic HTML)...");
  await api("PATCH", `/api/threads/${thread1.id}/status`, {
    status: "Resolved",
  });

  log("phase-4", "Human confirms timer cleanup...");
  await api("POST", `/api/threads/${thread2.id}/comments`, {
    author_type: "Human",
    body: "The cleanup function approach is clean. Resolved.",
  });
  log("phase-4", "Resolving thread 2 (setInterval cleanup)...");
  await api("PATCH", `/api/threads/${thread2.id}/status`, {
    status: "Resolved",
  });

  log("phase-4", "Agent replies to CSS custom properties thread...");
  await api("POST", `/api/threads/${thread3.id}/comments`, {
    author_type: "Agent",
    body: "Partially addressed — I introduced CSS custom properties for bg/text colors with the dark mode toggle. The accent color (#ff6600) is still hardcoded in a few places. Want me to extract that too?",
  });
  await api("POST", `/api/threads/${thread3.id}/comments`, {
    author_type: "Human",
    body: "Yes please, extract --accent as well. Can do it in the next revision.",
  });

  await pause("Check the UI — two threads are resolved, one still open with discussion.");

  // -----------------------------------------------------------------------
  // Phase 5: Final polish — Revision 3
  // -----------------------------------------------------------------------
  banner("Phase 5 — Revision 3: Contact Form");

  log("phase-5", "Modifying files for revision 3 (contact form, validation)...");
  writeFiles(repoPath, REV3_FILES);

  log("phase-5", "Creating revision 3...");
  const rev3 = await api<{ revision_number: number; file_count: number }>(
    "POST",
    `/api/reviews/${review.id}/revisions`,
    {
      trigger: "Manual",
      message: "Add contact form with client-side validation",
    },
  );
  log("phase-5", `Revision ${rev3.revision_number} created (${rev3.file_count} files)`);

  await pause("Check the UI — three revisions now. Try the interdiff view.");

  // -----------------------------------------------------------------------
  // Phase 6: Close the review
  // -----------------------------------------------------------------------
  banner("Phase 6 — Close Review");

  log("phase-6", "Resolving final thread...");
  await api("PATCH", `/api/threads/${thread3.id}/status`, {
    status: "Resolved",
  });

  log("phase-6", "Closing the review...");
  await api("PATCH", `/api/reviews/${review.id}/status`, {
    status: "Closed",
  });

  log("phase-6", "Review closed.");

  banner("Scenario Complete");
  console.log(`\n  Review ID: ${review.id}`);
  console.log(`  Repo path: ${repoPath}`);
  console.log(`  View at:   ${BASE_URL}\n`);

  rl.close();
}

main().catch((err) => {
  console.error(`\n  \x1b[31mError: ${err.message}\x1b[0m\n`);
  rl.close();
  process.exit(1);
});
