# A 90s/00s DECtalk text-to-speech script for linux and windows

- Generate TTS and play it through your mic in-game

# Requirements
**Linux**: 
* [dectalk](https://github.com/dectalk/dectalk)
* paplay `apt-get install pulseaudio-utils`

**Windows**: 
* [dectalk - old version](https://git.botox.bz/CSSZombieEscape/Torchlight3/src/branch/master/dectalk)
* pygame `pip3 install pygame`

# Scripts
**Windows**:
- dectalk = the path to dectalk - say.exe
- cwd     = the current working directory, make sure to put the correct directory or dectalk dictionary will not be loaded
- also make sure to create a folder called sounds, where the temporary sounds will be outputted

```python
import os
import asyncio
import random
from pygame import mixer

def main(args):
    message_struct = args['message'] # This is the message dict
    message = message_struct['message'] # This is the chat message
    asyncio.run(Say(message))
    pass

async def Say(text):
    # check for bad words
    if "badword" in text:
        return

    #if "badword2" in text:
    #    return

    # path to decatlk - say.exe
    dectalk  = r"C:\Custom\say.exe"

    # output the speech to a wav file
    filename = fr"sounds\{random.randint(1111, 9999)}.wav"

    # setup the command and arguments
    command  = f'"{dectalk}" -w "{filename}" "{text.replace('"', "")}"'

    # setup the current working directory
    cwd      = r"C:\Custom"

    filetoplay = fr"{cwd}\{filename}"

    # execute the command
    proc = await asyncio.create_subprocess_shell(
        command,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        cwd=cwd
    )
    await proc.communicate()
    
    # setup the output device
    mixer.init(devicename='CABLE Input (VB-Audio Virtual Cable)')
    
    # load the audio file
    mixer.music.load(filetoplay)
    
    # play it
    mixer.music.play()

    # wait until the audio is played
    while mixer.music.get_busy():
        pass

    # unload and remove the file
    mixer.music.unload()
    os.remove(filetoplay)
```

**Linux**:
- dectalk = the path to dectalk - say
- paplay  = path to paplay
- paplay_arg3 = name of the virtual mic
- lang  = can be "us, uk, gr, sp, la, fr"

```python
import os
import asyncio
import random

def main(args):
    message_struct = args['message'] # This is the message dict
    message = message_struct['message'] # This is the chat message
    asyncio.run(Say(message))
    pass

async def Say(text):
    # check for bad words
    if "badword" in text:
        return

    #if "badword2" in text:
    #    return

    # path to decatlk - say
    dectalk = "/home/username/Desktop/Dectalk/say"

    # output the speech to a wav file
    filename = f"/home/username/Desktop/Dectalk/sounds/{random.randint(1111, 9999)}.wav"
    
    # lang dictionary
    lang            = "us"
    
    # setup the command arguments
    dectalk_arg     = '-l'
    dectalk_arg2    = lang
    dectalk_arg3    = '-a'
    dectalk_arg4    = text
    dectalk_arg5    = '-fo'
    dectalk_arg6    = filename

    # execute the command
    proc = await asyncio.create_subprocess_exec(
        dectalk,
        dectalk_arg, dectalk_arg2, dectalk_arg3, dectalk_arg4, dectalk_arg5, dectalk_arg6
    )
    await proc.communicate()

    # play it with paplay
    paplay  = "/usr/bin/paplay"
    
    # setup the command arguments
    paplay_arg     = filename
    paplay_arg2    = '--device'
    paplay_arg3    = 'virtmic'

    # execute the command
    proc2 = await asyncio.create_subprocess_exec(
        paplay,
        paplay_arg, paplay_arg2, paplay_arg3
    )
    await proc2.communicate()

    # remove the file
    os.remove(filename)
```