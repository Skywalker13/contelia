if (/android/i.test(navigator.userAgent)) {
  window.location.href =
    "intent://${window.location.host}${window.location.pathname}#Intent;scheme=http;end";
}
