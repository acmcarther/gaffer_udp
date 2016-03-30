Feature: Sending Packets

  Background:
    Given a normal socket on 4979
    And a gaffer socket on 5989

  Scenario: Sending a first packet from the gaffer socket
    When the gaffer socket on 5989 sends a payload to 4979 matching:
      | 2 | 4 | 6 | 8 |
    Then the normal socket on 4979 receives a CompleteGafferPacket from 5989 matching:
      | seq       | 0       |
      | ack_seq   | 0       |
      | ack_field | 0       |
      | payload   | 2 4 6 8 |

  Scenario: Sending a first packet from the normal socket
    When the normal socket on 4979 sends a CompleteGafferPacket to 5989 matching:
      | seq       | 3        |
      | ack_seq   | 0        |
      | ack_field | 0        |
      | payload   | 3 6 9 12 |
    Then the gaffer socket on 5989 receives a payload from 4979 matching:
      | 3 | 6 | 9 | 12 |
