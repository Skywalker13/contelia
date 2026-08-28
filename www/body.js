if (window.location.search.includes("msg_text=")) {
  window.history.replaceState({}, "", window.location.pathname);
}

const pendingActions = new Map();

function updateUI() {
  const count = pendingActions.size;
  const applyBtn = document.getElementById("applyBtn");
  const pendingCount = document.getElementById("pendingCount");

  if (count > 0) {
    applyBtn.classList.add("visible");
    pendingCount.style.display = "block";
    pendingCount.textContent = count + " modification(s) en attente";
  } else {
    applyBtn.classList.remove("visible");
    pendingCount.style.display = "none";
  }
}

function toggleStory(btn, folderName) {
  const card = btn.closest(".book-card");
  const isCurrentlyDisabled = card.dataset.disabled === "true";
  const currentAction = pendingActions.get(folderName);

  if (currentAction) {
    pendingActions.delete(folderName);
    card.classList.remove(
      "pending-enable",
      "pending-disable",
      "pending-delete"
    );

    const pendingBadge = card.querySelector(".pending-badge");
    if (pendingBadge) {
      pendingBadge.remove();
    }
  } else {
    const action = isCurrentlyDisabled ? "enable" : "disable";
    pendingActions.set(folderName, { type: action, folder: folderName });

    if (action === "enable") {
      card.classList.add("pending-enable");
      card.querySelector(".book-title").innerHTML +=
        '<span class="pending-badge">→ ACTIVATION</span>';
    } else {
      card.classList.add("pending-disable");
      card.querySelector(".book-title").innerHTML +=
        '<span class="pending-badge">→ DÉSACTIVATION</span>';
    }
  }

  updateUI();
}

function deleteStory(btn, folderName, storyTitle) {
  const card = btn.closest(".book-card");
  const currentAction = pendingActions.get(folderName);

  if (currentAction) {
    pendingActions.delete(folderName);
    card.classList.remove(
      "pending-enable",
      "pending-disable",
      "pending-delete"
    );

    const pendingBadge = card.querySelector(".pending-badge");
    if (pendingBadge) {
      pendingBadge.remove();
    }
  } else {
    pendingActions.set(folderName, { type: "delete", folder: folderName });
    card.classList.add("pending-delete");

    const existingBadge = card.querySelector(".pending-badge");
    if (existingBadge) {
      existingBadge.remove();
    }
    card.querySelector(".book-title").innerHTML +=
      '<span class="pending-badge">→ SUPPRESSION</span>';
  }

  updateUI();
}

// Handle form submission
document.getElementById("batchForm")?.addEventListener("submit", function (e) {
  if (pendingActions.size === 0) {
    e.preventDefault();
    return;
  }

  const actionsArray = Array.from(pendingActions.values());
  document.getElementById("batchActions").value = JSON.stringify(actionsArray);
});

function formatBytesJS(bytes) {
  if (bytes < 1024) {
    return bytes + "B";
  }
  if (bytes < 1048576) {
    return Math.round(bytes / 1024) + "KB";
  }
  return (bytes / 1048576).toFixed(1) + "MB";
}

document.getElementById("uploadForm").addEventListener("submit", function (e) {
  e.preventDefault();

  const fileInput = document.getElementById("fileInput");
  if (!fileInput.files.length) {
    return;
  }

  const file = fileInput.files[0];
  const formData = new FormData();
  formData.append("archive", file, file.name);

  const submitBtn = document.getElementById("submitBtn");
  const progress = document.getElementById("uploadProgress");
  const progressBar = document.getElementById("progressBar");
  const progressText = document.getElementById("progressText");

  submitBtn.disabled = true;
  progress.classList.add("active");
  progressBar.classList.remove("indeterminate");
  progressBar.style.width = "0%";

  const xhr = new XMLHttpRequest();

  let switchedToDecompress = false;

  xhr.upload.addEventListener("progress", function (evt) {
    if (!evt.lengthComputable) {
      return;
    }

    const percent = Math.round((evt.loaded / evt.total) * 100);
    progressBar.style.width = percent + "%";
    progressText.textContent =
      percent +
      "% — " +
      formatBytesJS(evt.loaded) +
      " / " +
      formatBytesJS(evt.total);

    if (percent >= 100 && !switchedToDecompress) {
      switchedToDecompress = true;
      progressBar.classList.add("indeterminate");
      progressText.textContent = "Décompression en cours...";
    }
  });

  xhr.upload.addEventListener("load", function () {
    if (switchedToDecompress) {
      return;
    }

    switchedToDecompress = true;
    progressBar.classList.add("indeterminate");
    progressText.textContent = "Décompression en cours...";
  });

  xhr.onload = function () {
    let type = "error";
    let text = "Réponse inattendue du serveur.";
    try {
      const data = JSON.parse(xhr.responseText);
      type = data.type || type;
      text = data.text || text;
    } catch (e) {
      /* ... */
    }

    const url = new URL(window.location.pathname, window.location.origin);
    if (text) {
      url.searchParams.set("msg_type", type);
      url.searchParams.set("msg_text", text);
    }
    window.location.href = url.toString();
  };

  xhr.onerror = function () {
    progress.classList.remove("active");
    submitBtn.disabled = false;
    progressText.textContent = "";
    alert("Erreur réseau pendant l'envoi.");
  };

  xhr.open("POST", window.location.pathname + "?ajax=1", true);
  xhr.send(formData);
});
