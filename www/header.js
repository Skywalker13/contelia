if (
  /android/i.test(navigator.userAgent) &&
  !sessionStorage.getItem("redirected")
) {
  sessionStorage.setItem("redirected", "true");
  window.location.href = `intent://${window.location.host}${window.location.pathname}#Intent;scheme=http;end`;
}
