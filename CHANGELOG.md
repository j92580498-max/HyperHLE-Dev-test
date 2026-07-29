# Changelog

## Unreleased for 0.3.0-alpha.1

- Rebrand the fork as tapHLE across the executable, crates, runtime resources,
  internal symbols, tests, and packaging.
- Define Windows game compatibility as the product focus, with macOS retained
  for development convenience and Android outside the active support scope.
- Add AI-agent contribution guidance, provenance rules, upstream quarantine
  instructions, game-focused issue forms, and repository policy checks.
- Add a simple self-service guide for people using coding agents to improve a
  game they care about.
- Move the current compatibility record to the live tapHLEdb service while
  retaining legacy JSON records for migration and offline validation.
- Add exact Archive.org compatibility provenance and verified three-star
  in-game milestones for Ricky, Baby Monkey, Cops & Robbers, and SPYmouse HD.
- Add MP3 streaming support, AudioQueue OpenAL source cleanup, clean
  secondary-thread shutdown, and state-aware EAGL frame capture for
  agent-driven Windows testing.
- Start hundreds of apps that previously died before showing anything. Support
  was chosen by surveying where 1501 apps stop and fixing the most common
  causes: Objective-C autorelease-pool and exception-handler entry points,
  runtime class creation, integer-division and C++ allocation builtins, and a
  large set of Foundation, UIKit, Core Animation, Media Player and Audio
  Session constants that apps read during startup.
- Stop treating ordinary runtime conditions as fatal. Retaining a constant
  string, asking for a class that does not exist, an out-of-range array index,
  a mapping the allocator cannot satisfy, and messaging an object that was
  released too many times are all defined or recoverable on a device, and each
  previously ended the app.
- Add UITabBarController, UIToolbar, UIProgressView, UIPageControl,
  NSURLProtocol, NSCondition and NSUbiquitousKeyValueStore, and let nibs decode
  bar items, colours in more colour spaces, and navigation and scroll view
  properties.
- Verified three-star in-game milestones for Tap Tap Revenge 2, Omium,
  Parachute Panic HD and Scoops, and a two-star start for JellyCar.
- Start more apps still: glGetIntegerv now answers for every parameter rather
  than only integer ones, and Objective-C block retention, run-time class
  creation, Core Graphics colour components, Foundation allocation zones and
  extended-attribute calls all work.
- Stop ending the app over ordinary input. A locale identifier carrying a
  script or a numeric region, an OpenGL ES parameter name the backend does not
  know, a nib that produces no objects or sets no view, a message whose
  declared argument types disagree with the receiver's, and an application
  started without a delegate class are all things a device accepts, and each
  one previously aborted at launch.

### Known limitations

- Throwing an Objective-C or C++ exception is still unimplemented and reports
  itself clearly; registering the handlers, which is what almost every app
  actually needed, works.
- UITabBarController and UIToolbar hold and report their contents but are not
  drawn, so an app relying on the tab bar to change tabs stays on the first tab.
- Zoom, pattern colours and content gravity are stored and reported back rather
  than applied.

Earlier release history belongs to the upstream touchHLE project and can be
found in its repository history.
