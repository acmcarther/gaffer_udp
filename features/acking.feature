Feature: Acking Packets

  Background:
    Given a normal socket on 4979
    And a gaffer socket on 5979
    And the gaffer socket on 5979 sends a payload to 4979
    And the gaffer socket on 5979 sends a payload to 4979
    And the gaffer socket on 5979 sends a payload to 4979
    And the normal socket on 4979 receives a CompleteGafferPacket from 5979
    And the normal socket on 4979 receives a CompleteGafferPacket from 5979
    And the normal socket on 4979 receives a CompleteGafferPacket from 5979

  Scenario: Acking the normal sockets packets
    When the normal socket on 4979 sends a CompleteGafferPacket to 5979 matching:
      | seq       | 3 |
      | ack_seq   | 2 |
      | ack_field | 3 |
      | payload   |   |
    And the normal socket on 4979 sends a CompleteGafferPacket to 5979 matching:
      | seq       | 6 |
      | ack_seq   | 2 |
      | ack_field | 3 |
      | payload   |   |
    And the gaffer socket on 5979 receives a payload from 4979
    And the gaffer socket on 5979 receives a payload from 4979
    And the gaffer socket on 5979 sends a payload to 4979
    Then the normal socket on 4979 receives a CompleteGafferPacket from 5979 matching:
      | seq       | 3 |
      | ack_seq   | 6 |
      | ack_field | 4 |
      | payload   |   |

  Scenario: Acking the normal sockets packets around zero
    When the normal socket on 4979 sends a CompleteGafferPacket to 5979 matching:
      | seq       | 0 |
      | ack_seq   | 2 |
      | ack_field | 3 |
      | payload   |   |
    And the normal socket on 4979 sends a CompleteGafferPacket to 5979 matching:
      | seq       | 65534 |
      | ack_seq   | 2     |
      | ack_field | 3     |
      | payload   |       |
    And the gaffer socket on 5979 receives a payload from 4979
    And the gaffer socket on 5979 receives a payload from 4979
    And the gaffer socket on 5979 sends a payload to 4979
    Then the normal socket on 4979 receives a CompleteGafferPacket from 5979 matching:
      | seq       | 3 |
      | ack_seq   | 0 |
      | ack_field | 2 |
      | payload   |   |
