example rust + pam auth + file caps
===

caps + setuid example of elevating to auth with pam

```
Process timeline
────────────────────────────────────────────────────────────────────
start (invoking uid=1000)   ← file caps: cap_setuid,cap_setgid in permitted
  │
  ├─ parse args              (uid 1000, no effective caps)
  ├─ read password from tty  (uid 1000)
  ├─ build PAM handle        (uid 1000)
  │
  ├─ elevate_to_root()  ──── capng::change_id(0, 0)
  │     └─ uid/gid → 0, caps: still only setuid+setgid effective
  │
  ├─ pam::authenticate()  ←── ONLY call that runs as root
  │
  ├─ drop_privileges()  ──── capng::clear() + capng::change_id(1000, 1000)
  │     └─ uid/gid → 1000, ALL capabilities cleared (empty permitted set)
  │
  ├─ inspect result          (uid 1000, zero capabilities)
  └─ exit
```

## Pam service file

May need more attention...

```text
auth    required    pam_unix.so     nodelay nullok
account required    pam_unix.so
```
