## History

In this file I will try to add important milestones/achievements in a happy format.

#### 2019-04-23
Big milestone reached today!! I have managed to figure how too respond to the players
browsing requests. That is one big step towards being able to choose a track in the player interface
that this application will upload.

There is much more work required to be able to manage multiple players. That is because the
RemoteDBServer currently use a hardcoded TCP Port in the response.

#### 2019-03-24
Current state of this project is that players have started to respond to commnication.
I'm currently working on implementing the RPC server. Some work has been made on
analyzing how that communication looks like. A first naive implementation should
not be super far away.

#### 2019-03-14
General cleaning and re-arranging code. Also renamed to project to `TermDJ`.
Name is short for Terminal DJ.

#### 2019-03-13
Managed to automate network discovery. I.E. find the interface/network address that CD-players
broadcast on. Use that network and send out initial linking / broadcasting packages.

#### 2019-03-12
Manged to finally get my XDJ-700 to respond to the network traffic.
The key part is to send packages in the right order and to package correct IP-address and MAC address.

I also decided to purge the old master branch and rebuild this from the ground and up.
Test coverage wasn't the greatest on the old branch. Notice: the old branch is kept and is named `old-master`.
