Feature: Reliability in variable network conditions

  Demo Application:
    Our test program sends math challenges to a client, who sends back answers
    Problems are divided into "series". A client will never answer problems of series prior to the latest one it has solved.
    Example: if a client receives problem 10 from series C, it will never answer a problem 9 from series C.

  Definitions:
    - Application Domains:
      The amount of parallel "data types" in the pipeline. For our demo application, this is the "series"
      The number of data types will contribute to determining how network variability impacts potential applications

    - Packet Interchange:
      A single roughly equitable exchange of messages between client and server

    - Application Reliability:
      For the demo application, this is defined as the percentage of problems answered by the client

    - Packet Delay:
      This is a factor used by the simulated network to perturb the transmission time of packets using a normal distribution,
      where mean = 0.5*PD, and standard deviation of 1

    - Drop Percentage:
      This is the probability that any particular packet will be dropped

  Background:
    Given a network
    And a server
    And a client


  Scenario Outline: Reliability guarantees
    Given <DOMAIN_COUNT> parallel domains in the networked application
    And packet delay variation of 0ms
    #And unlimited bandwidth
    And <DROP_PERCENTAGE>% of packets are dropped
    When 1000 packet interchanges are run
    Then the application exhibits <RELIABILITY_PERCENTAGE>% "reliability"

    # TODO: Determine the values for this
    Examples:
      | DOMAIN_COUNT | DROP_PERCENTAGE | RELIABILITY_PERCENTAGE |
      | 1            | 0               | 100                    |
      | 1            | 1               | 100                    |
      | 1            | 2               | 100                    |
      | 1            | 5               | 100                    |
      | 1            | 10              | 100                    |
      | 5            | 0               | 100                    |
      | 5            | 2               | 100                    |
      | 5            | 5               | 100                    |
      | 5            | 10              | 100                    |
      | 10           | 0               | 100                    |
      | 10           | 5               | 100                    |
      | 10           | 10              | 100                    |


  Scenario Outline: Ordering guarantees
    Given <DOMAIN_COUNT> parallel domains in the networked application
    And packet delay variation of <PDV_MS>ms
    #And unlimited bandwidth
    And 0% of packets are dropped
    When 1000 packet interchanges are run
    Then the application exhibits <RELIABILITY_PERCENTAGE>% "reliability"

    # TODO: Determine the values for this
    Examples:
      | DOMAIN_COUNT | PDV_MS | RELIABILITY_PERCENTAGE |
      | 1            | 0      | 100                    |
      | 1            | 1      | 100                    |
      | 1            | 2      | 100                    |
      | 1            | 5      | 100                    |
      | 1            | 10     | 100                    |
      | 5            | 0      | 100                    |
      | 5            | 2      | 100                    |
      | 5            | 5      | 100                    |
      | 5            | 10     | 100                    |
      | 10           | 0      | 100                    |
      | 10           | 5      | 100                    |
      | 10           | 10     | 100                    |
