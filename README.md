# Bevy Robotic Dojo  

An example of using bevy + bevy_urdf + bevy_rl as an environemnt to train neural network to control a robot.

This is an WIP. Environment runs in realtime with 100hz pause to receive updates from a NN. Robot isn't being reset on reset request because body isn't despawned from rapier context. Reward signal is robot velocity in positive direction of ground plane