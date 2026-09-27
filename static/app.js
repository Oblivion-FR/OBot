// Tooltips for `[data-tooltip]`, positioned on the page so the scrolling server rail can't clip them.
// Listeners live on `document`, so they keep working after htmx swaps the body.
(() => {
  let tooltip;

  const show = (target) => {
    if (!tooltip || !tooltip.isConnected) {
      tooltip = document.createElement("div");
      tooltip.className = "tooltip";
      tooltip.setAttribute("role", "tooltip");
      document.body.append(tooltip);
    }
    const rect = target.getBoundingClientRect();
    tooltip.textContent = target.dataset.tooltip;
    tooltip.style.top = `${rect.top + rect.height / 2}px`;
    tooltip.style.left = `${rect.right + 14}px`;
    tooltip.classList.add("visible");
  };
  const hide = () => tooltip?.classList.remove("visible");

  document.addEventListener("mouseover", (event) => {
    const target = event.target.closest("[data-tooltip]");
    target ? show(target) : hide();
  });
  document.addEventListener("focusin", (event) => {
    const target = event.target.closest("[data-tooltip]");
    target ? show(target) : hide();
  });
  document.addEventListener("focusout", hide);
  document.addEventListener("scroll", hide, true);
  document.addEventListener("htmx:beforeSwap", hide);
})();
