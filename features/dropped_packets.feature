Feature: Packet drop recovery

  Rules:
    - Protocol specifies that packets send ACK, and ACK_FIELD which contains 32 past packet acks
    - If a packet is unacked for the 33 subsequent packets, it is considered dropped and is resubmitted
    - TODO: If a packet is unacked for a TBD time T, it is considered dropped and is resubmitted
    - TODO: If a packet is dropped TBD N times, it is dropped permanently


  Background:
    Given a gaffer socket


  Scenario: Resubmission of dropped packet after the sliding ack window passes
    When the socket sends a packet with payload "foo"
    And the socket sends a packet with payload "bar"
    And 2 packets are dropped
    And the socket sends 33 packets
    And 33 packets are received
    And the socket is sent 1 packet to provide ack information
    When the socket sends a packet with payload "baz"
    Then 3 packets are received
    And the socket's last 3 payloads include:
      | foo |
      | bar |
      | baz |
