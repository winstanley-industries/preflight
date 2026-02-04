const REVIEW_PATTERN = /^\/reviews\/([^/]+)\/?$/;

interface Route {
  page: "list" | "review";
  reviewId: string | null;
}

function parseRoute(pathname: string): Route {
  const match = pathname.match(REVIEW_PATTERN);
  if (match) {
    return { page: "review", reviewId: match[1] };
  }
  return { page: "list", reviewId: null };
}

let current = $state<Route>(parseRoute(window.location.pathname));

function handlePopState() {
  current = parseRoute(window.location.pathname);
}

if (typeof window !== "undefined") {
  window.addEventListener("popstate", handlePopState);
}

export function navigate(path: string) {
  history.pushState(null, "", path);
  current = parseRoute(path);
}

export function getRoute(): Route {
  return current;
}
