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

## 3. Initial state and action availability

- [ ] There is no single generic progress bar. Separate civic factors are visible.
- [ ] Only **File a formal complaint** and **Mobilise the neighbourhood** are initially shown for the drainage mission.
- [ ] **Visit the municipal office**, **File an information request**, councillor, media and court actions are absent rather than displayed as disabled buttons.
- [ ] The Phaser city has hotspots only for the actions in the visible action list.

## 4. First stochastic action

- [ ] Select “File a formal complaint”.
- [ ] An outcome dialog appears.
- [ ] The dialog lists each changed civic factor as a percentage delta and its new value, for example **Evidence strength +8% → 21%**.
- [ ] Money, energy and days decrease.
- [ ] Evidence strength and any other declared factors change.
- [ ] A case-history entry appears.
- [ ] **File a formal complaint** disappears after it is used once.
- [ ] **Visit the municipal office** and **File an information request** appear because the complaint changed the case state.
- [ ] The exact outcome can differ in a newly created game.

## 5. Strategy paths

Try at least two different approaches.

### Formal-record path

- [ ] File a complaint.
- [ ] Visit the office or file an information request when available.
- [ ] Build evidence strength and institutional pressure.
- [ ] Confirm court action appears only after its factor and state requirements are met.

### Collective-pressure path

- [ ] Mobilise the neighbourhood once.
- [ ] Confirm public support and movement unity increase, then the action disappears.
- [ ] Confirm councillor and media actions become available when their requirements are met.

### Informal-payment path

- [ ] Confirm the action is absent before the office has responded to a filed complaint.
- [ ] Use it only after the required case state exists.
- [ ] Confirm integrity falls even when other factors improve.
- [ ] Confirm an informal payment can fail or create additional costs.

## 6. Evidence-stage progression

Start **The Examination Scandal**.

- [ ] The evidence stage begins as **Unverified**.
- [ ] **Verify leaked answer sheets** changes it to **Independently verified**, then disappears.
- [ ] **Corroborate the leak** appears next and advances it to **Corroborated**.
- [ ] **Confirm chain of custody** appears next and advances it to **Chain-of-custody confirmed**.
- [ ] **Prepare a legally admissible record** appears next and advances it to **Legally admissible**.
- [ ] An earlier verification action never reappears merely because it was clicked again.
- [ ] If the whistleblower-packet event occurs, **Verify whistleblower evidence** appears as a new, context-specific action.

## 7. Multi-factor win and loss

- [ ] Confirm no one repeated action can produce a win.
- [ ] Meet the ending's thresholds across at least two factors and confirm the matching victory.
- [ ] Confirm high evidence alone is insufficient when public, media or institutional thresholds are still unmet.
- [ ] Restart the same citizen and confirm the state returns to the initial values with a new stochastic context.
- [ ] In another run, spend resources inefficiently until time or energy runs out and confirm “Mission failed”.

## 8. Persistence

- [ ] Refresh the page; the current session returns.
- [ ] Close and reopen the browser; the current session returns.
- [ ] Run `STOP_GAME.bat`, then `START_GAME.bat`; the session returns.
- [ ] Run `RESET_GAME_DATA.bat`; old sessions no longer return.

## 9. Duplicate and repeated-action protection

The interface disables an action only while its request is running. For API-level verification:

- [ ] Send the same `client_action_id` twice. The second response is identical and does not deduct resources again.
- [ ] Submit the already consumed action with a new `client_action_id`. The server returns HTTP 400, the turn and resources do not change, and the action remains absent from the UI.

## 10. Phone layout

- [ ] Open the computer’s LAN URL on a phone.
- [ ] No horizontal scrolling is required.
- [ ] Action buttons are readable and easy to tap.
- [ ] Add the game to the home screen and launch it in standalone mode.
