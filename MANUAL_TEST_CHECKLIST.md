# Manual test checklist

For an immediate no-install test, open `PLAY_WITHOUT_DOCKER.html`. Repeat persistence and API-specific checks using Docker.

Record failures with the selected citizen, turn number and visible status message.

## 1. Startup

- [ ] `START_GAME.bat` completes without an error.
- [ ] `http://localhost:8080` opens.
- [ ] The header shows a saved-session count rather than “Server unavailable”.
- [ ] Three citizen profiles are visible.

## 2. Citizen profiles

- [ ] Ramesh has more money and community support.
- [ ] Meera starts with stronger documentation and influence.
- [ ] Salim starts with less money but more energy and community support.
- [ ] Selecting a profile opens the drainage mission.

## 3. Initial state and action locks

- [ ] Progress begins at 0%.
- [ ] “Visit the municipal office” is locked until a complaint exists.
- [ ] “File an information request” is locked initially.
- [ ] “Approach the ward councillor” is locked without sufficient community support.
- [ ] “Escalate through local media” and “File a public-interest petition” are locked initially.

## 4. First stochastic action

- [ ] Select “File a formal complaint”.
- [ ] An outcome dialog appears.
- [ ] Money, energy and days decrease.
- [ ] Documentation and progress increase.
- [ ] A case-history entry appears.
- [ ] The exact outcome can differ in a newly created game.

## 5. Strategy paths

Try at least two different approaches.

### Formal-record path

- [ ] File a complaint.
- [ ] Visit the office or file an information request when available.
- [ ] Build documentation to at least 45.
- [ ] Confirm court action eventually becomes available after enough progress.

### Collective-pressure path

- [ ] Mobilise the neighbourhood repeatedly.
- [ ] Confirm community support increases.
- [ ] Confirm councillor and media actions become available when their requirements are met.

### Informal-payment path

- [ ] Confirm the action is unavailable before a traceable file exists.
- [ ] Use it after progress exceeds 5.
- [ ] Confirm integrity falls even when progress improves.
- [ ] Confirm an informal payment can fail or create additional costs.

## 6. Win and loss

- [ ] Reach 100% progress and confirm “Mission completed”.
- [ ] Restart the same citizen and confirm the state returns to the initial values with a new stochastic context.
- [ ] In another run, spend resources inefficiently until time or energy runs out and confirm “Mission failed”.

## 7. Persistence

- [ ] Refresh the page; the current session returns.
- [ ] Close and reopen the browser; the current session returns.
- [ ] Run `STOP_GAME.bat`, then `START_GAME.bat`; the session returns.
- [ ] Run `RESET_GAME_DATA.bat`; old sessions no longer return.

## 8. Duplicate-action protection

The interface disables an action while the request is running. For API-level verification, send the same `client_action_id` twice. The second response should be identical and should not deduct resources again.

## 9. Phone layout

- [ ] Open the computer’s LAN URL on a phone.
- [ ] No horizontal scrolling is required.
- [ ] Action buttons are readable and easy to tap.
- [ ] Add the game to the home screen and launch it in standalone mode.
