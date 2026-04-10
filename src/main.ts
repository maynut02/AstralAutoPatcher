import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();
const PATCH_EVENT_NAME = "patch://event";

const PROGRESS_CENTER_ICON = {
  success:
    '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="progress-ring__status-icon lucide lucide-check-icon lucide-check"><path d="M20 6 9 17l-5-5"/></svg>',
  error:
    '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="progress-ring__status-icon progress-ring__status-icon--error lucide lucide-x-icon lucide-x"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>',
} as const;

type LogState = "pending" | "current" | "done" | "error";

type LaunchContext = {
  mode: "patch" | "remove" | "install" | string;
  target: string;
  protocolArg?: string | null;
};

type PatchEventPayload = {
  stepIndex: number;
  title: string;
  state: "current" | "done" | "error";
  message: string;
  detail?: string | null;
  progress: number;
};

type ProgressController = {
  setProgress: (value: number) => void;
  markSuccess: () => void;
  markError: () => void;
};

type ProtocolMode = "patch" | "remove";

const PATCH_STEP_TITLES = [
  "프로그램 버전 확인",
  "게임 설치 폴더 찾기",
  "로컬 다운로드 폴더 확인",
  "한글패치 파일 준비",
  "한글패치 적용",
  "결과",
] as const;

const INSTALL_STEP_TITLES = ["프로그램 버전 확인", "프로그램 설치", "커스텀 프로토콜 등록", "결과"] as const;
const REMOVE_STEP_TITLES = ["로컬 다운로드 경로 확인", "제거 규칙 조회", "한글 패치 제거", "결과"] as const;

function isPatchAutoExitCountdown(detail?: string | null): boolean {
  if (!detail) {
    return false;
  }

  return /\d+\s*초 후 프로그램이 자동으로 종료됩니다\.?\s*$/u.test(detail.trim());
}

function parseProtocolContext(protocolArg?: string | null): { mode: ProtocolMode; route: string } | null {
  if (!protocolArg) {
    return null;
  }

  const matched = protocolArg.match(/^astral:\/\/(patch|remove)\/([^/?#]+)/i);
  if (!matched) {
    return null;
  }

  const mode = matched[1].toLowerCase() as ProtocolMode;
  const route = matched[2].trim();
  if (!route) {
    return null;
  }

  return { mode, route };
}

function updateTitlebarBadge(context: LaunchContext) {
  const badgeContainerEl = document.querySelector<HTMLElement>(".app-titlebar__badges");
  if (!badgeContainerEl) {
    return;
  }

  const protocolContext = parseProtocolContext(context.protocolArg);
  const badges: string[] = [];

  if (!protocolContext) {
    badges.push("설치 모드");
  } else if (protocolContext.mode === "patch") {
    const route = context.target || protocolContext.route;
    badges.push("패치 모드", route);
  } else {
    badges.push("제거 모드", protocolContext.route);
  }

  badgeContainerEl.replaceChildren(
    ...badges.map((label) => {
      const badgeEl = document.createElement("span");
      badgeEl.className = "app-titlebar__badge";
      badgeEl.textContent = label;
      return badgeEl;
    }),
  );
}

const LOG_ICON_MAP: Record<LogState, { className: string; body: string }> = {
  pending: {
    className: "lucide lucide-ellipsis-icon lucide-ellipsis",
    body: '<circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/>',
  },
  current: {
    className: "lucide lucide-loader-icon lucide-loader",
    body: '<path d="M12 2v4"/><path d="m16.2 7.8 2.9-2.9"/><path d="M18 12h4"/><path d="m16.2 16.2 2.9 2.9"/><path d="M12 18v4"/><path d="m4.9 19.1 2.9-2.9"/><path d="M2 12h4"/><path d="m4.9 4.9 2.9 2.9"/>',
  },
  done: {
    className: "lucide lucide-check-icon lucide-check",
    body: '<path d="M20 6 9 17l-5-5"/>',
  },
  error: {
    className: "lucide lucide-circle-x-icon lucide-circle-x",
    body: '<circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/>',
  },
};

function initCircularProgress(): ProgressController | null {
  const progressEl = document.querySelector<HTMLElement>(".progress-ring");
  if (!progressEl) {
    return null;
  }

  const barEl = progressEl.querySelector<SVGCircleElement>(".progress-ring__bar");
  if (!barEl) {
    return null;
  }

  const radius = Number(barEl.getAttribute("r") ?? "0");
  const circumference = 2 * Math.PI * radius;

  const valueEl = progressEl.querySelector<HTMLElement>(".progress-ring__value");
  const setProgress = (rawValue: number) => {
    const progress = Math.max(0, Math.min(100, Number.isFinite(rawValue) ? rawValue : 0));
    barEl.style.strokeDasharray = `${circumference}`;
    barEl.style.strokeDashoffset = `${circumference * (1 - progress / 100)}`;
    progressEl.setAttribute("aria-valuenow", `${Math.round(progress)}`);

    valueEl?.replaceChildren(`${Math.round(progress)}%`);
  };

  const initialValue = Number(progressEl.dataset.progress ?? "0");
  setProgress(initialValue);

  return {
    setProgress: (value: number) => {
      progressEl.classList.remove("progress-ring--success", "progress-ring--error");
      setProgress(value);
    },
    markSuccess: () => {
      progressEl.classList.remove("progress-ring--error");
      progressEl.classList.add("progress-ring--success");
      setProgress(100);
      if (valueEl) {
        valueEl.innerHTML = PROGRESS_CENTER_ICON.success;
      }
    },
    markError: () => {
      progressEl.classList.remove("progress-ring--success");
      progressEl.classList.add("progress-ring--error");
      if (valueEl) {
        valueEl.innerHTML = PROGRESS_CENTER_ICON.error;
      }
    },
  };
}

function bindWindowControls() {
  document.querySelector<HTMLButtonElement>("#window-minimize")?.addEventListener("click", () => {
    void appWindow.minimize();
  });

  document.querySelector<HTMLButtonElement>("#window-close")?.addEventListener("click", () => {
    void appWindow.close();
  });

  document.querySelector<HTMLElement>(".app-titlebar__title")?.addEventListener("mousedown", (event) => {
    if (event.buttons !== 1) {
      return;
    }

    if (event.detail === 2) {
      return;
    }

    void appWindow.startDragging();
  });
}

function bindBrowserFeatureGuards() {
  document.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });

  window.addEventListener("keydown", (event) => {
    const key = event.key.toUpperCase();
    const ctrlOrMeta = event.ctrlKey || event.metaKey;

    const isDevtoolsShortcut =
      key === "F12" ||
      (ctrlOrMeta && event.shiftKey && ["I", "J", "C", "K"].includes(key)) ||
      (ctrlOrMeta && key === "U");

    if (!isDevtoolsShortcut) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
  });
}

function setLogItemIcon(item: HTMLElement, state: LogState) {
  const svg = item.querySelector<SVGSVGElement>("svg");
  if (!svg) {
    return;
  }

  const icon = LOG_ICON_MAP[state];
  svg.setAttribute("class", icon.className);
  svg.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  svg.setAttribute("width", "24");
  svg.setAttribute("height", "24");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "2");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.innerHTML = icon.body;
}

function createLogItem(title: string): HTMLElement {
  const item = document.createElement("div");
  item.className = "app-log-list__item";
  item.innerHTML =
    '<svg aria-hidden="true"></svg><div class="app-log-list__text"><span class="app-log-list__title"></span><span class="app-log-list__meta"></span></div>';

  const titleEl = item.querySelector<HTMLElement>(".app-log-list__title");
  if (titleEl) {
    titleEl.textContent = title;
  }

  return item;
}

function renderLogItems(container: HTMLElement, titles: readonly string[]): HTMLElement[] {
  const items = titles.map((title) => createLogItem(title));
  container.replaceChildren(...items);
  return items;
}

function scrollLogListToBottom(container: HTMLElement) {
  container.scrollTop = container.scrollHeight;
}

function scrollLogListToBottomSoon(container: HTMLElement, focusItem?: HTMLElement) {
  const sync = () => {
    scrollLogListToBottom(container);
    focusItem?.scrollIntoView({ block: "end", inline: "nearest" });
  };

  // Run multiple times to follow height growth caused by max-height transition.
  sync();
  requestAnimationFrame(sync);
  window.setTimeout(sync, 180);
  window.setTimeout(sync, 380);
}

function bindLogListAutoScroll(container: HTMLElement) {
  const observer = new MutationObserver((mutations) => {
    const hasNewLogItem = mutations.some((mutation) =>
      Array.from(mutation.addedNodes).some(
        (node) => node instanceof HTMLElement && node.classList.contains("app-log-list__item"),
      ),
    );

    if (!hasNewLogItem) {
      return;
    }

    requestAnimationFrame(() => {
      scrollLogListToBottom(container);
    });
  });

  observer.observe(container, { childList: true });
}

function ensureLogTitleElement(item: HTMLElement): HTMLElement | null {
  const textContainer = item.querySelector<HTMLElement>(".app-log-list__text");
  if (!textContainer) {
    return null;
  }

  const existing = textContainer.querySelector<HTMLElement>(".app-log-list__title");
  if (existing) {
    return existing;
  }

  const firstSpan = textContainer.querySelector<HTMLElement>(":scope > span");
  if (!firstSpan) {
    return null;
  }

  firstSpan.classList.add("app-log-list__title");
  return firstSpan;
}

function ensureLogMetaElement(item: HTMLElement): HTMLElement | null {
  const textContainer = item.querySelector<HTMLElement>(".app-log-list__text");
  if (!textContainer) {
    return null;
  }

  const existing = textContainer.querySelector<HTMLElement>(".app-log-list__meta");
  if (existing) {
    return existing;
  }

  const meta = document.createElement("span");
  meta.className = "app-log-list__meta";
  textContainer.append(meta);
  return meta;
}

function setLogItemContent(item: HTMLElement, title: string, detail?: string | null) {
  const titleEl = ensureLogTitleElement(item);
  if (titleEl) {
    titleEl.textContent = title;
  }

  const metaEl = ensureLogMetaElement(item);
  if (!metaEl) {
    return;
  }

  if (detail && detail.trim().length > 0) {
    metaEl.textContent = detail;
    metaEl.style.display = "block";
  } else {
    metaEl.textContent = "";
    metaEl.style.display = "none";
  }
}

function setLogItemState(item: HTMLElement, state: LogState) {
  item.classList.remove(
    "app-log-list__item--visible",
    "app-log-list__item--done",
    "app-log-list__item--current",
    "app-log-list__item--error",
  );

  if (state === "pending") {
    setLogItemIcon(item, "pending");
    return;
  }

  item.classList.add("app-log-list__item--visible");

  if (state === "done") {
    item.classList.add("app-log-list__item--done");
    setLogItemIcon(item, "done");
    return;
  }

  if (state === "current") {
    item.classList.add("app-log-list__item--current");
    setLogItemIcon(item, "current");
    return;
  }

  item.classList.add("app-log-list__item--error");
  setLogItemIcon(item, "error");
}

function applyPatchEvent(
  payload: PatchEventPayload,
  logItems: HTMLElement[],
  progressController: ProgressController | null,
  logListContainer: HTMLElement,
) {
  const currentIndex = Math.max(0, Math.min(logItems.length - 1, payload.stepIndex - 1));
  const currentItem = logItems[currentIndex];

  logItems.forEach((item, index) => {
    if (index < currentIndex) {
      setLogItemState(item, "done");
      return;
    }

    if (index > currentIndex) {
      setLogItemState(item, "pending");
      return;
    }

    if (payload.state === "done") {
      setLogItemState(item, "done");
      return;
    }

    if (payload.state === "error") {
      setLogItemState(item, "error");
      return;
    }

    setLogItemState(item, "current");
  });

  setLogItemContent(currentItem, payload.message, payload.detail);
  progressController?.setProgress(payload.progress);

  if (payload.state === "done" && payload.progress >= 100) {
    progressController?.markSuccess();
  }

  if (payload.state === "error") {
    progressController?.markError();
  }

  const skipAutoScrollForCountdown = payload.state === "done" && isPatchAutoExitCountdown(payload.detail);

  if (!skipAutoScrollForCountdown) {
    scrollLogListToBottomSoon(logListContainer, currentItem);
  }
}

function prepareLogList(logItems: HTMLElement[]) {
  logItems.forEach((item) => {
    ensureLogTitleElement(item);
    ensureLogMetaElement(item);
    setLogItemState(item, "pending");
  });
}

async function initPatchRuntime(progressController: ProgressController | null) {
  const logListContainer = document.querySelector<HTMLElement>(".app-log-list");
  if (!logListContainer) {
    return;
  }

  bindLogListAutoScroll(logListContainer);

  try {
    const context = await invoke<LaunchContext>("get_launch_context");
    updateTitlebarBadge(context);

    const stepTitles =
      context.mode === "patch"
        ? PATCH_STEP_TITLES
        : context.mode === "remove"
          ? REMOVE_STEP_TITLES
          : INSTALL_STEP_TITLES;
    const logItems = renderLogItems(logListContainer, stepTitles);
    prepareLogList(logItems);
    scrollLogListToBottomSoon(logListContainer);

    await listen<PatchEventPayload>(PATCH_EVENT_NAME, (event) => {
      applyPatchEvent(event.payload, logItems, progressController, logListContainer);
    });

    if (context.mode === "install") {
      progressController?.setProgress(0);
      await invoke("start_install_workflow");
      return;
    }

    if (context.mode === "remove") {
      await invoke("start_remove_workflow", { target: context.target });
      return;
    }

    await invoke("start_patch_workflow", { target: context.target });
  } catch (error) {
    const logItems = renderLogItems(logListContainer, PATCH_STEP_TITLES);
    prepareLogList(logItems);
    const first = logItems[0];
    setLogItemState(first, "error");
    setLogItemContent(first, "패치 워크플로우 시작 실패", String(error));
    progressController?.markError();
  }
}

window.addEventListener("DOMContentLoaded", () => {
  bindBrowserFeatureGuards();
  bindWindowControls();
  const progressController = initCircularProgress();
  void initPatchRuntime(progressController);
});
