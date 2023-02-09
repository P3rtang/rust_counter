# Counter TUI

This is a simple counter app that can be run in any modern terminal emulator  
The app is mainly geared towards logging progress on shiny hunting pokemon,  
but it should work just as well for logging anything else  

## features
Some of the major features are:
- calculating odds when hunting pokemon
- keeping track of hunt phases
- optional keylogger (WARNING read section on the keylogger for safety)

## installing
- Linux
    ```
    make install
    ```

- Windows
    run the install.bat batch file

- Other platforms
    ```
    cargo build --release
    ```

## keylogger
This section will talk about the keylogger and it's safety  
The main reason this program includes a keylogger is to act on inputs without the window having focus  
If you'd want to increase the counter by pressing plus without a keylogger  
you first need to bring the window the counter is running in back in focus and only then can it register the keypress  
The keylogger bypasses this so you can use the counter in the background  

- Linux  
  On linux the keylogging function can only be activated by running the program as super user,  
  this is done by reading from /dev/input/  
  be careful when running any program with sudo,  
  
  This being said the keylogger can run without sudo, but this in my opinion is more dangerous,  
  and that is by adding your user to the input group with  
  `usermod -aG input $USER`  
  but as stated above I do not recommend this as any program can now read /dev/input/ without sudo  

- other platforms  
  the keylogger is not available yet on other platforms  

## GOALS
- [x] basic functionality
- [x] hunt phases
- [x] keylogger
- [x] keylogger auto keyboard detect
- [x] settings menu framework
- [ ] better testing 
- [ ] opening multiple counters at once 
