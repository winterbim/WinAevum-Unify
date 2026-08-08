Run Aevum doctor — hard mission self-check (never silent failure).

```bash
unify doctor --mission "${AEVUM_MISSION:?set AEVUM_MISSION}"
```

Expect `AEVUM_DOCTOR_OK`. Soft warnings (missing slopcheck) are fine; hard failures block the agent.
