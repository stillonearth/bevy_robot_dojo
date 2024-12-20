# Bevy Robotic Dojo  

An experimental Reinforcement Learning (RL) environment built using the Bevy game engine, designed to reimplement functionality similar to MUJOCO but with a focus on leveraging Rust's strengths. The primary objective of this project is to develop robust and flexible RL environments that support advanced scenarios without compromising on physics accuracy. 

## Goals 

- _Environment Logic in Rust_ : Implement environment logic entirely in Rust to take advantage of its performance benefits, enabling the creation of more complex and demanding RL tasks.
- _Multi-Agent Interaction_ : Facilitate robot-versus-robot interaction in a dojo setting, where each agent has access to both sensor data and visual input (camera images).
- _Robotic Sumo Challenge_ : Enable robots to engage in sumo-style combat within a defined arena, with the goal of pushing an opponent out of a circular boundary.
- _Physics Simulation in Rust World_ : While MUJOCO provides high-quality physics simulations in its own world, Bevy Robotic Dojo aims to run the entire simulation within a Rust-based ECS (Entity Component System) environment using Bevy. This allows for more complex and flexible scenarios compared to merely rendering MUJOCO's physics.
     

## Features 

**Core Components** 

- _Bevy Engine Integration_ : Utilizes Bevy's ECS architecture for efficient and modular game development.
- _Rust-based Environment Logic_ : Written entirely in Rust to ensure high performance and robustness.
- _Physics Simulation_ : Leverages Avian to simulate realistic interactions between robots.
- _Sensor and Camera Data_ : Provides both sensory input data (camera) and visual information from simulated cameras.
     

## Example Scenario: Robotic Sumo 

In the robotic sumo scenario: 

- Two robots are placed within a circular boundary.
- Each robot receives sensor data about its surroundings and camera images to make decisions on movement and strategy.
- The objective is for each robot to use this information to push the other out of the circle.

## Approximations

- Camera input is just bevy render No optical distortions or simulation of real camera
- _Physics Accuracy and Sim-to-Real Transfer_ : The accuracy of physics simulations and their applicability in real-world scenarios have not been evaluated yet. Additional research is needed to compare the correctness and fidelity of Bevy’s physics system with that of MUJOCO.

     
