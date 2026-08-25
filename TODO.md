# QSONaut Server TODO

## FEATURE REQUEST
Look for existing packages that provide straightforward support and configuration options.
https://github.com/nicksbar/QSONaut-Server-Proprietary <-- this is checked out in local workspace, empty.
I created this PRIVATE repo for this proprietary work, i imagine the repo as an ability to include in the public package to extend the capability for my hosted implementation that has these features. Without it, its designed for a free/local/personal use.


- Oauth, google, others
- Payments, paypal, other

Should consider
* Individual club payment setup
* Fees
* Club public landing page
* Email verification and invitation delivery after an access request is approved
* Rate limiting / abuse monitoring for public access and HamDB lookup endpoints
* Turnstile or equivalent hosted CAPTCHA option for deployments that need stronger
  abuse resistance than the server-side technical challenge

Are there other aspects of this that should be kept out of public space?

## COMPLETED / ACCESS REQUEST FOUNDATION

* Public home-screen access request form with callsign, HamDB validation,
  optional club, referral source, and email address.
* Persisted server-side technical challenges with a ten-minute expiry and three
  attempts; the answer is never sent to the browser.
* Administrator access-request queue with approve/reject decisions. Approval
  now atomically creates a member account with a generated temporary password;
  the password is returned once to the administrator for secure handoff.

## FEATURE REQUEST
Activity can be marked so that so that it can be know if we are operating within close proximity to another, like for field day where we are all in a radius. With the possibility of multiple transmitters and antennas all going at the same time - we may not identify IMD right away, if at all. I bet we could track the connected radio frequencies and calculate the likelyhood of that occurring based on the number of transmitters and the frequencies/modes/bands they operating on and provide a early warning to our qsonaut client

## ISSUE
We need to version our server and client in qsonaut.
So we can report mismatch and get client updated.

# Public/hosted edition boundary

- [x] Keep personal activity, shared club/activity setup, and contest support in the public server.
- [x] Enforce the public five-club ceiling atomically and advertise edition capabilities.
- [x] Scaffold the private companion as a library extension rather than a standalone server.
- [ ] Move hosted organization administration into the proprietary extension assembly.
- [ ] Add proprietary OAuth/OIDC, billing/payment, email/invitation, abuse controls, and hosted CAPTCHA adapters.
