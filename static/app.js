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

// A re-verified row comes back with a fresh notice row, drop the previous one first
const dropNotice = (row) => document.querySelector(`tr[data-for="${row.id}"]`)?.remove();

// Error messages come translated from the page, with English as a fallback
const MESSAGES = document.body.dataset;
const SESSION_EXPIRED =
  MESSAGES.errorSessionExpired ?? "Your session expired, reload the page to log in again.";
const GENERIC_ERROR = MESSAGES.errorGeneric ?? "Something went wrong, check the bot logs.";
const UNREACHABLE = MESSAGES.errorUnreachable ?? "Couldn't reach the panel, try again.";

// Plain text client errors are the panel's translated refusals. Anything else would be a whole
// page, or a server error only described in English.
const errorText = (response, text) =>
  response.status < 500 && (response.headers.get("content-type") ?? "").startsWith("text/plain") && text
    ? text
    : GENERIC_ERROR;

const showRowError = (row, text) => {
  dropNotice(row);
  const notice = document.createElement("tr");
  notice.className = "notice-row notice-error";
  notice.dataset.for = row.id;
  const cell = notice.insertCell();
  cell.colSpan = row.cells.length;
  cell.textContent = text;
  row.after(notice);
};

document.addEventListener("htmx:beforeSwap", (event) => {
  const target = event.detail.target;
  if (!(target instanceof HTMLTableRowElement && target.id)) return;
  // A redirect to the login page means the session is gone, don't put that page in the table
  if (event.detail.xhr.responseURL && !event.detail.xhr.responseURL.endsWith(event.detail.pathInfo.requestPath)) {
    event.detail.shouldSwap = false;
    showRowError(target, SESSION_EXPIRED);
    return;
  }
  if (event.detail.xhr.status >= 400) {
    event.detail.shouldSwap = false;
    const xhr = event.detail.xhr;
    const plain = xhr.status < 500 && (xhr.getResponseHeader("content-type") ?? "").startsWith("text/plain");
    showRowError(target, plain && xhr.responseText ? xhr.responseText : GENERIC_ERROR);
    return;
  }
  dropNotice(target);
});

document.addEventListener("htmx:sendError", (event) => {
  const target = event.detail.target;
  if (target instanceof HTMLTableRowElement && target.id) {
    showRowError(target, UNREACHABLE);
  }
});

// Verify-on-behalf dialog. It posts with fetch because its target row changes per member.
(() => {
  let url;
  let rowId;

  document.addEventListener("click", (event) => {
    const opener = event.target.closest("[data-verify-url]");
    const dialog = document.getElementById("verify-dialog");
    if (opener && dialog) {
      url = opener.dataset.verifyUrl;
      rowId = opener.closest("tr").id;
      dialog.querySelector("[data-member-name]").textContent = opener.dataset.memberName;
      dialog.querySelector("[data-form-error]").textContent = "";
      dialog.querySelector("form").reset();
      dialog.showModal();
      dialog.querySelector("input[name=username]").focus();
    }
    if (event.target.closest("[data-close]")) {
      event.target.closest("dialog")?.close();
    }
  });

  document.addEventListener("submit", async (event) => {
    const form = event.target;
    if (form.id !== "verify-form") return;
    event.preventDefault();
    const dialog = form.closest("dialog");
    const error = form.querySelector("[data-form-error]");
    const submit = form.querySelector("button[type=submit]");
    submit.disabled = true;
    error.textContent = "";
    try {
      const response = await fetch(url, {
        method: "POST",
        body: new URLSearchParams(new FormData(form)),
      });
      const text = await response.text();
      if (response.redirected) {
        error.textContent = SESSION_EXPIRED;
        return;
      }
      if (!response.ok) {
        error.textContent = errorText(response, text);
        return;
      }
      const row = document.getElementById(rowId);
      if (row) {
        dropNotice(row);
        const template = document.createElement("template");
        template.innerHTML = `<table><tbody>${text}</tbody></table>`;
        const rows = [...template.content.querySelectorAll("tbody > tr")];
        row.replaceWith(...rows);
        rows.forEach((newRow) => htmx.process(newRow));
      }
      dialog.close();
    } catch {
      error.textContent = UNREACHABLE;
    } finally {
      submit.disabled = false;
    }
  });
})();

// Column toggles of the members table. The hidden columns go in a cookie so the next page load
// renders them hidden right away; rows swapped in later only need the table's classes.
document.addEventListener("change", (event) => {
  const toggle = event.target.closest("[data-column-toggle]");
  if (!toggle) return;
  const table = document.querySelector("table.members");
  table?.classList.toggle(`hide-${toggle.dataset.columnToggle}`, !toggle.checked);
  const hidden = [...document.querySelectorAll("[data-column-toggle]")]
    .filter((box) => !box.checked)
    .map((box) => box.dataset.columnToggle);
  document.cookie = `obot_hidden_columns=${hidden.join(",")}; path=/; max-age=31536000; samesite=lax`;
});

// Close the columns menu when clicking elsewhere
document.addEventListener("click", (event) => {
  document.querySelectorAll("details.column-menu[open]").forEach((menu) => {
    if (!menu.contains(event.target)) menu.open = false;
  });
});

// Edit dialogs of the rules page. The button carries the form's action and, in `data-field-*`
// attributes, the value of each field by name.
document.addEventListener("click", (event) => {
  const opener = event.target.closest("[data-edit-dialog]");
  const dialog = opener && document.getElementById(opener.dataset.editDialog);
  if (!dialog) return;
  const form = dialog.querySelector("form");
  form.action = opener.dataset.action;
  for (const [key, value] of Object.entries(opener.dataset)) {
    if (!key.startsWith("field")) continue;
    const name = key.charAt(5).toLowerCase() + key.slice(6);
    const field = form.elements.namedItem(name);
    if (field) field.value = value;
  }
  dialog.showModal();
});

// Dragging a rule onto another group moves it there
(() => {
  let dragged;
  const zoneOf = (event) => event.target.closest?.("[data-drop-group]");
  const clearTargets = () =>
    document.querySelectorAll(".drop-target").forEach((zone) => zone.classList.remove("drop-target"));

  document.addEventListener("dragstart", (event) => {
    dragged = event.target.closest?.("[data-rule-id]");
    if (!dragged) return;
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", dragged.dataset.ruleId);
    dragged.classList.add("dragging");
  });
  document.addEventListener("dragend", () => {
    dragged?.classList.remove("dragging");
    dragged = undefined;
    clearTargets();
  });
  document.addEventListener("dragover", (event) => {
    const zone = zoneOf(event);
    if (!dragged || !zone || zone.dataset.dropGroup === dragged.dataset.groupId) return;
    event.preventDefault();
    clearTargets();
    zone.classList.add("drop-target");
  });
  document.addEventListener("drop", (event) => {
    const zone = zoneOf(event);
    if (!dragged || !zone || zone.dataset.dropGroup === dragged.dataset.groupId) return;
    event.preventDefault();
    // A plain form post: the page comes back with the rule moved, or with the error
    const form = document.createElement("form");
    form.method = "post";
    form.action = dragged.dataset.moveUrl;
    const group = document.createElement("input");
    group.type = "hidden";
    group.name = "group_id";
    group.value = zone.dataset.dropGroup;
    form.append(group);
    document.body.append(form);
    form.submit();
  });
})();
